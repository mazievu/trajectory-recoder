#[test]
fn test_f15_typing_burst_coalescing() {
    let characters = ["H", "e", "l", "l", "o", " ", "W", "o", "r", "l", "d"];
    let mut burst = String::new();
    for ch in characters {
        burst.push_str(ch);
    }

    assert_eq!(burst, "Hello World");
    assert_eq!(characters.len(), 11);
}
