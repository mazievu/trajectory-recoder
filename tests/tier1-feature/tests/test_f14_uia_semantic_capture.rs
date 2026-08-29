use harness_app::uia_provider::{UiaBoundingRect, UiaControlType, UiaElementNode};

#[test]
fn test_f14_uia_element_hit_test_and_property_extraction() {
    let mut root = UiaElementNode::new(
        "wnd_main",
        "Main Window",
        "Window",
        UiaControlType::Window,
        UiaBoundingRect {
            left: 0.0,
            top: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );
    root.children.push(UiaElementNode::new(
        "btn_save",
        "Save",
        "Button",
        UiaControlType::Button,
        UiaBoundingRect {
            left: 100.0,
            top: 200.0,
            width: 80.0,
            height: 30.0,
        },
    ));

    let hit = root.hit_test(120.0, 210.0);
    assert!(hit.is_some());
    let element = hit.unwrap();
    assert_eq!(element.automation_id, "btn_save");
    assert_eq!(element.control_type, UiaControlType::Button);
}
