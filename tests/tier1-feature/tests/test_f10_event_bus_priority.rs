#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    P0InputCanonical = 0,
    P1Window = 1,
    P2DomUia = 2,
    P3Screenshot = 3,
    P4Video = 4,
}

#[test]
fn test_f10_priority_drop_order_under_saturation() {
    assert!(EventPriority::P4Video > EventPriority::P3Screenshot);
    assert!(EventPriority::P3Screenshot > EventPriority::P2DomUia);
    assert!(EventPriority::P2DomUia > EventPriority::P1Window);
    assert!(EventPriority::P1Window > EventPriority::P0InputCanonical);
}
