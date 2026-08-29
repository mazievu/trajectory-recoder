#[cfg(windows)]
pub mod native {
    use crate::model::{UiaAncestorInfo, UiaElementInfo, control_type_id_to_name};
    use core_types::metadata::BoundingRect;
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker,
    };
    use windows::core::{BSTR, Result as WinResult};

    pub struct NativeUiaContext {
        automation: IUIAutomation,
        tree_walker: IUIAutomationTreeWalker,
    }

    impl NativeUiaContext {
        pub fn init() -> WinResult<Self> {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                let automation: IUIAutomation =
                    CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
                let tree_walker = automation.ControlViewWalker()?;
                Ok(Self {
                    automation,
                    tree_walker,
                })
            }
        }

        pub fn element_from_point(&self, x: i32, y: i32) -> Option<UiaElementInfo> {
            let pt = POINT { x, y };
            let elem = unsafe { self.automation.ElementFromPoint(pt).ok()? };
            Some(self.extract_element_info(&elem))
        }

        pub fn element_from_handle(&self, hwnd: HWND) -> Option<UiaElementInfo> {
            let elem = unsafe { self.automation.ElementFromHandle(hwnd).ok()? };
            Some(self.extract_element_info(&elem))
        }

        pub fn get_focused_element(&self) -> Option<UiaElementInfo> {
            let elem = unsafe { self.automation.GetFocusedElement().ok()? };
            Some(self.extract_element_info(&elem))
        }

        fn extract_element_info(&self, elem: &IUIAutomationElement) -> UiaElementInfo {
            let name = unsafe {
                elem.CurrentName()
                    .ok()
                    .map(|b: BSTR| b.to_string())
                    .filter(|s| !s.is_empty())
            };

            let automation_id = unsafe {
                elem.CurrentAutomationId()
                    .ok()
                    .map(|b: BSTR| b.to_string())
                    .filter(|s| !s.is_empty())
            };

            let class_name = unsafe {
                elem.CurrentClassName()
                    .ok()
                    .map(|b: BSTR| b.to_string())
                    .filter(|s| !s.is_empty())
            };

            let framework_id = unsafe {
                elem.CurrentFrameworkId()
                    .ok()
                    .map(|b: BSTR| b.to_string())
                    .filter(|s| !s.is_empty())
            };

            let control_type_id = unsafe { elem.CurrentControlType().map(|id| id.0).unwrap_or(0) };
            let control_type = control_type_id_to_name(control_type_id).to_string();

            let is_enabled = unsafe { elem.CurrentIsEnabled().unwrap_or_default().as_bool() };
            let is_keyboard_focusable = unsafe {
                elem.CurrentIsKeyboardFocusable()
                    .unwrap_or_default()
                    .as_bool()
            };
            let is_password = unsafe { elem.CurrentIsPassword().unwrap_or_default().as_bool() };
            let is_offscreen = unsafe { elem.CurrentIsOffscreen().unwrap_or_default().as_bool() };

            let help_text = unsafe {
                elem.CurrentHelpText()
                    .ok()
                    .map(|b: BSTR| b.to_string())
                    .filter(|s| !s.is_empty())
            };

            let bounding_rect = unsafe {
                elem.CurrentBoundingRectangle()
                    .ok()
                    .map(|r: RECT| BoundingRect::new(r.left, r.top, r.right, r.bottom))
            };

            // Walk up to 3 ancestor levels
            let ancestors = self.walk_ancestors(elem, 3);

            UiaElementInfo {
                name,
                control_type,
                control_type_id,
                automation_id,
                class_name,
                framework_id,
                bounding_rect,
                is_enabled,
                is_keyboard_focusable,
                is_password,
                is_offscreen,
                value: None,
                help_text,
                ancestors,
            }
        }

        fn walk_ancestors(
            &self,
            elem: &IUIAutomationElement,
            max_levels: u32,
        ) -> Vec<UiaAncestorInfo> {
            let mut ancestors = Vec::new();
            let mut current = elem.clone();

            for level in 1..=max_levels {
                let parent = match unsafe { self.tree_walker.GetParentElement(&current) } {
                    Ok(p) => p,
                    Err(_) => break,
                };

                let name = unsafe {
                    parent
                        .CurrentName()
                        .ok()
                        .map(|b: BSTR| b.to_string())
                        .filter(|s| !s.is_empty())
                };
                let automation_id = unsafe {
                    parent
                        .CurrentAutomationId()
                        .ok()
                        .map(|b: BSTR| b.to_string())
                        .filter(|s| !s.is_empty())
                };
                let class_name = unsafe {
                    parent
                        .CurrentClassName()
                        .ok()
                        .map(|b: BSTR| b.to_string())
                        .filter(|s| !s.is_empty())
                };
                let framework_id = unsafe {
                    parent
                        .CurrentFrameworkId()
                        .ok()
                        .map(|b: BSTR| b.to_string())
                        .filter(|s| !s.is_empty())
                };
                let control_type_id =
                    unsafe { parent.CurrentControlType().map(|id| id.0).unwrap_or(0) };
                let control_type = control_type_id_to_name(control_type_id).to_string();

                ancestors.push(UiaAncestorInfo {
                    level,
                    name,
                    control_type,
                    control_type_id,
                    automation_id,
                    class_name,
                    framework_id,
                });

                current = parent;
            }

            ancestors
        }
    }

    impl Drop for NativeUiaContext {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
