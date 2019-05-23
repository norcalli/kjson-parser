pub mod section;
pub mod tokenizer;
pub mod validator;

#[cfg(test)]
mod tests {

    // #[test]
    // fn it_parses_a_stream() {
    //     let tests = [
    //         (r"1 1", [1, 1]),
    //         (r#"1 "123""#, ["1", r#""123""#]),
    //         (r#"1"123""#, ["1", r#""123""#]),
    //         (r#"[1]{"a": null}"#, ["[1]", r#"{"a":null}"#]),
    //     ];
    // }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
