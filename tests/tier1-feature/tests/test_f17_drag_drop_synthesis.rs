use harness_app::controls::drag_view::{DragDropZone, DragState};

#[test]
fn test_f17_drag_drop_state_lifecycle() {
    let mut zone = DragDropZone::new("item_01", "folder_assets");
    assert_eq!(zone.current_state, DragState::Idle);

    zone.start_drag("file_payload.png");
    assert_eq!(zone.current_state, DragState::Dragging);

    let dropped = zone.complete_drop();
    assert_eq!(zone.current_state, DragState::DroppedSuccess);
    assert_eq!(dropped, Some("file_payload.png".to_string()));
}
