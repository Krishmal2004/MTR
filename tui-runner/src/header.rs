pub fn header_lines(title: &str) -> Vec<String> {
    let upper = title.to_uppercase();
    let width = upper.chars().count() + 4;
    let top = format!("+{}+", "-".repeat(width));
    let mid = format!("|  {}  |", upper);
    let bottom = top.clone();
    vec![top, mid, bottom]
}
pub fn print_header(title: &str) {
    for line in header_lines(title) {
        println!("{}", line);
    }
}