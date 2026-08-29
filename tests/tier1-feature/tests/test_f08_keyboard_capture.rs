#[test]
fn test_f08_modifier_flags_combination() {
    let ctrl_mask = 1 << 0;
    let alt_mask = 1 << 1;
    let shift_mask = 1 << 2;

    let ctrl_c = ctrl_mask;
    let ctrl_alt_del = ctrl_mask | alt_mask;

    assert_eq!(ctrl_c & ctrl_mask, ctrl_mask);
    assert_eq!(ctrl_alt_del & (ctrl_mask | alt_mask), ctrl_mask | alt_mask);
    assert_eq!(ctrl_alt_del & shift_mask, 0);
}
