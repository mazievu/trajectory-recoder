use crate::drag_drop::DragDropStateMachine;
use crate::scroll::ScrollBurstAggregator;
use crate::typing::TypingBurstAggregator;
use core_types::action::{
    ActionParameters, ActionType, CanonicalAction, CanonicalActionBuilder, ClickParams,
    ClipboardParams, FileOperationParams, WindowLifecycleParams,
};
use core_types::event::{RawEvent, RawEventPayload, RawWindowEvent};
use core_types::id::{GlobalEventId, SessionId};
use core_types::metadata::{
    ApplicationContext, ContextMetadata, MouseButton, Point2D, TargetMetadata,
    WindowContext,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Central correlation engine transforming multi-source raw events into CanonicalActions.
pub struct CorrelationEngine {
    session_id: SessionId,
    user_id: String,
    machine_id: String,
    global_seq: Arc<AtomicU64>,
    session_event_seq: Arc<AtomicU64>,
    typing_aggregator: TypingBurstAggregator,
    scroll_aggregator: ScrollBurstAggregator,
    drag_drop_sm: DragDropStateMachine,
    current_context: ContextMetadata,
}

impl CorrelationEngine {
    pub fn new(
        session_id: impl Into<SessionId>,
        user_id: impl Into<String>,
        machine_id: impl Into<String>,
        global_seq: Arc<AtomicU64>,
    ) -> Self {
        let u_id = user_id.into();
        let m_id = machine_id.into();
        let initial_context = ContextMetadata {
            user_id: u_id.clone(),
            machine_id: m_id.clone(),
            ..Default::default()
        };

        Self {
            session_id: session_id.into(),
            user_id: u_id,
            machine_id: m_id,
            global_seq,
            session_event_seq: Arc::new(AtomicU64::new(1)),
            typing_aggregator: TypingBurstAggregator::default(),
            scroll_aggregator: ScrollBurstAggregator::default(),
            drag_drop_sm: DragDropStateMachine::default(),
            current_context: initial_context,
        }
    }

    pub fn set_session_id(&mut self, session_id: SessionId) {
        self.session_id = session_id;
        self.session_event_seq.store(1, Ordering::SeqCst);
    }

    pub fn update_window_context(&mut self, window_event: &RawWindowEvent) {
        self.current_context.process_name = window_event.process_name.clone();
        self.current_context.process_id = window_event.pid;
        self.current_context.window_title = window_event.window_title.clone();
        self.current_context.window_handle = window_event.hwnd;
        self.current_context.monitor_id = window_event.monitor_id;

        self.current_context.application = ApplicationContext {
            process_name: window_event.process_name.clone(),
            pid: window_event.pid,
            executable_path: None,
            app_id: None,
            is_elevated: false,
        };

        self.current_context.window = WindowContext {
            hwnd: window_event.hwnd,
            title: window_event.window_title.clone(),
            bounds: window_event.bounds,
            is_maximized: false,
            is_minimized: false,
            is_foreground: window_event.event_type == "FOREGROUND",
            is_fullscreen: false,
            dpi: window_event.dpi,
        };
    }

    fn next_ids(&self) -> (u64, u64) {
        let g = self.global_seq.fetch_add(1, Ordering::Relaxed);
        let s = self.session_event_seq.fetch_add(1, Ordering::Relaxed);
        (g, s)
    }

    fn build_click_action(
        &self,
        event: &RawEvent,
        button: MouseButton,
        point: Point2D,
        monitor_id: u32,
        target: TargetMetadata,
        is_double: bool,
        ids: Option<(u64, u64)>,
    ) -> CanonicalAction {
        let (gid, sid) = ids.unwrap_or_else(|| self.next_ids());
        let action_type = match (button, is_double) {
            (MouseButton::Right, _) => ActionType::RightClick,
            (MouseButton::Middle, _) => ActionType::MiddleClick,
            (MouseButton::Left, true) => ActionType::DoubleClick,
            _ => ActionType::Click,
        };

        let click_params = ClickParams {
            button,
            click_count: if is_double { 2 } else { 1 },
            physical_coords: point,
            normalized_coords: point,
            monitor_id,
        };

        CanonicalActionBuilder::new(
            GlobalEventId::new(gid),
            self.session_id.clone(),
            sid,
            event.timestamp,
            action_type,
            ActionParameters::Click(click_params),
        )
        .target(target)
        .context(self.current_context.clone())
        .build()
    }

    /// Process a raw event and optionally produce one or more CanonicalActions.
    pub fn process_event(
        &mut self,
        event: &RawEvent,
        target_override: Option<TargetMetadata>,
    ) -> Vec<CanonicalAction> {
        let mut actions = Vec::new();
        let target = target_override.unwrap_or_default();

        match &event.payload {
            RawEventPayload::Mouse(mouse_event) => {
                let point = Point2D::new(
                    mouse_event.physical_x,
                    mouse_event.physical_y,
                    mouse_event.normalized_x,
                    mouse_event.normalized_y,
                );

                match mouse_event.event_type.as_str() {
                    "MOUSE_MOVE" => {
                        self.drag_drop_sm.on_mouse_move(point);
                    }
                    "MOUSE_DOWN" => {
                        self.drag_drop_sm.on_mouse_down(
                            event.timestamp,
                            mouse_event.button,
                            point,
                            target.clone(),
                            self.current_context.clone(),
                        );
                    }
                    "MOUSE_UP" => {
                        if self.drag_drop_sm.is_active_for(mouse_event.button) {
                            let (gid, sid) = self.next_ids();
                            match self.drag_drop_sm.on_mouse_up(
                                event.timestamp,
                                mouse_event.button,
                                point,
                                target.clone(),
                                &self.session_id,
                                gid,
                                sid,
                            ) {
                                Some(dd_action) => actions.push(dd_action),
                                None => actions.push(self.build_click_action(
                                    event,
                                    mouse_event.button,
                                    point,
                                    mouse_event.monitor_id,
                                    target,
                                    false,
                                    Some((gid, sid)),
                                )),
                            }
                        }
                    }
                    "CLICK" | "DOUBLE_CLICK" => {
                        let is_double = mouse_event.event_type == "DOUBLE_CLICK";
                        actions.push(self.build_click_action(
                            event,
                            mouse_event.button,
                            point,
                            mouse_event.monitor_id,
                            target,
                            is_double,
                            None,
                        ));
                    }
                    "MOUSE_WHEEL" => {
                        let (gid, sid) = self.next_ids();
                        if let Some(scroll_action) = self.scroll_aggregator.on_wheel(
                            event.timestamp,
                            mouse_event.delta_x,
                            mouse_event.delta_y,
                            target,
                            self.current_context.clone(),
                            &self.session_id,
                            gid,
                            sid,
                        ) {
                            actions.push(scroll_action);
                        }
                    }
                    _ => {}
                }
            }
            RawEventPayload::Keyboard(kb_event) => {
                let (gid, sid) = self.next_ids();
                let is_down = kb_event.event_type == "KEY_DOWN";
                if let Some(typing_action) = self.typing_aggregator.on_keystroke(
                    event.timestamp,
                    kb_event.vk_code,
                    &kb_event.key_name,
                    is_down,
                    target,
                    self.current_context.clone(),
                    &self.session_id,
                    gid,
                    sid,
                ) {
                    actions.push(typing_action);
                }
            }
            RawEventPayload::Window(win_event) => {
                self.update_window_context(win_event);
                let (gid, sid) = self.next_ids();
                let action_type = match win_event.event_type.as_str() {
                    "FOREGROUND" => ActionType::WindowSwitch,
                    "OPEN" => ActionType::WindowOpen,
                    "CLOSE" => ActionType::WindowClose,
                    _ => ActionType::WindowSwitch,
                };

                let win_params = WindowLifecycleParams {
                    hwnd: win_event.hwnd,
                    event_type: win_event.event_type.clone(),
                    process_name: win_event.process_name.clone(),
                    window_title: win_event.window_title.clone(),
                };

                let action = CanonicalActionBuilder::new(
                    GlobalEventId::new(gid),
                    self.session_id.clone(),
                    sid,
                    event.timestamp,
                    action_type,
                    ActionParameters::Window(win_params),
                )
                .context(self.current_context.clone())
                .build();

                actions.push(action);
            }
            RawEventPayload::Clipboard(clip_event) => {
                let (gid, sid) = self.next_ids();
                let clip_params = ClipboardParams {
                    operation: "COPY".to_string(),
                    content_type: clip_event.format.clone(),
                    byte_length: clip_event.byte_length,
                    hash_sha256: clip_event.hash_sha256.clone(),
                    source_app: clip_event.source_hwnd.map(|h| format!("0x{:X}", h)),
                    destination_app: None,
                    redacted_preview: None,
                };

                let action_type = ActionType::Copy;

                let action = CanonicalActionBuilder::new(
                    GlobalEventId::new(gid),
                    self.session_id.clone(),
                    sid,
                    event.timestamp,
                    action_type,
                    ActionParameters::Clipboard(clip_params),
                )
                .context(self.current_context.clone())
                .build();

                actions.push(action);
            }
            RawEventPayload::File(file_event) => {
                let (gid, sid) = self.next_ids();
                let path_obj = std::path::Path::new(&file_event.file_path);
                let file_name = path_obj
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                let extension = path_obj
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();

                let file_params = FileOperationParams {
                    operation: file_event.action.clone(),
                    file_path: file_event.file_path.clone(),
                    file_name,
                    extension,
                    size_bytes: None,
                };

                let action_type = match file_event.action.as_str() {
                    "CREATED" => ActionType::FileCreate,
                    "MODIFIED" => ActionType::FileSave,
                    "DELETED" => ActionType::FileDelete,
                    "RENAMED" => ActionType::FileRename,
                    _ => ActionType::FileSave,
                };

                let action = CanonicalActionBuilder::new(
                    GlobalEventId::new(gid),
                    self.session_id.clone(),
                    sid,
                    event.timestamp,
                    action_type,
                    ActionParameters::File(file_params),
                )
                .context(self.current_context.clone())
                .build();

                actions.push(action);
            }
            _ => {}
        }

        actions
    }

    /// Periodic flush for debounced typing and scrolling bursts.
    pub fn periodic_flush(&mut self) -> Vec<CanonicalAction> {
        let mut actions = Vec::new();
        let (gid1, sid1) = self.next_ids();
        if let Some(t_action) = self.typing_aggregator.check_timeout(&self.session_id, gid1, sid1) {
            actions.push(t_action);
        }

        let (gid2, sid2) = self.next_ids();
        if let Some(s_action) = self.scroll_aggregator.check_timeout(&self.session_id, gid2, sid2) {
            actions.push(s_action);
        }

        actions
    }
}
