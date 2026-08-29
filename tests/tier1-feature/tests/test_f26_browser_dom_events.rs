use browser_events::BrowserDomEvent;

#[test]
fn test_f26_browser_dom_events_and_xpath() {
    let dom_event = BrowserDomEvent {
        tab_id: 1,
        url: "https://app.example.com/form".to_string(),
        page_title: "Registration Form".to_string(),
        event_type: "INPUT".to_string(),
        tag: "input".to_string(),
        role: Some("textbox".to_string()),
        visible_text: None,
        aria_label: Some("User Name".to_string()),
        element_id: Some("username-field".to_string()),
        class_name: Some("form-control".to_string()),
        href: None,
        placeholder: Some("Enter username".to_string()),
        input_type: Some("text".to_string()),
        value: Some("alice_smith".to_string()),
        css_selector: Some("#username-field".to_string()),
        xpath: Some("//input[@id='username-field']".to_string()),
        timestamp_ms: 1700000000,
        is_password: false,
        mutation_info: None,
    };

    let target = dom_event.to_target_metadata();
    assert_eq!(target.control_type.as_deref(), Some("input"));
    assert_eq!(target.automation_id.as_deref(), Some("username-field"));
    assert_eq!(target.value.as_deref(), Some("alice_smith"));
    assert_eq!(target.framework_id.as_deref(), Some("DOM"));
}
