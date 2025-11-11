#[test]
fn debug_collapse_continuations() {
    let s1 = "parallel \\\n    for schedule(static)";
    let s2 = "parallel\\\n    for\\\n    private(i)";

    let c1 = roup::lexer::collapse_line_continuations(s1);
    let c2 = roup::lexer::collapse_line_continuations(s2);

    println!("orig1=[{}] -> [{}]", s1, c1.as_ref());
    println!("orig2=[{}] -> [{}]", s2, c2.as_ref());

    assert!(c1.as_ref().contains("parallel"));
    assert!(c2.as_ref().contains("parallel"));
}
