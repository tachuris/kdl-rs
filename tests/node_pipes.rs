//! Tests for experimental node pipes. `a #> b` is equivalent to `b { a }`.

use kdl::{KdlDocument, KdlNode};

#[cfg(not(feature = "experimental-node-pipes"))]
#[test]
fn pipes_are_invalid_without_the_feature() {
    for text in ["a #> b", "a#>b", "a #>b", "a#> b"] {
        assert!(KdlDocument::parse(text).is_err(), "{text:?}");
    }
}

#[cfg(feature = "experimental-node-pipes")]
mod node_pipes {
    use super::*;
    use kdl::KdlEntry;

    fn parse(text: &str) -> KdlDocument {
        KdlDocument::parse(text).unwrap_or_else(|e| panic!("{text:?}: {e:?}"))
    }

    fn normalized(mut document: KdlDocument) -> KdlDocument {
        document.clear_format_recursive();
        document
    }

    /// Asserts that the pipe text and nested text parse into equivalent documents.
    #[track_caller]
    fn assert_equivalent(pipe: &str, nested: &str) {
        assert_eq!(
            normalized(parse(pipe)),
            normalized(parse(nested)),
            "{pipe:?} should be equivalent to {nested:?}"
        );
    }

    #[track_caller]
    fn assert_invalid(text: &str) {
        assert!(
            KdlDocument::parse(text).is_err(),
            "{text:?} should be invalid"
        );
    }

    #[track_caller]
    fn assert_round_trip(text: &str) {
        assert_eq!(parse(text).to_string(), text);
    }

    /// Asserts that the formatted document has no pipes and matches the nested text.
    #[track_caller]
    fn assert_formats_nested(document: &KdlDocument, nested: &str) {
        let text = document.to_string();
        assert!(!text.contains("#>"), "{text:?} should not contain pipes");
        assert_equivalent(&text, nested);
    }

    fn first_child(document: &mut KdlDocument) -> &mut KdlNode {
        &mut document.nodes_mut()[0]
            .children_mut()
            .as_mut()
            .expect("children")
            .nodes_mut()[0]
    }

    const EXAMPLE_PIPE: &str = r#"
app {
    route "/users" { handler list-users #> auth role=admin #> cache ttl=(s)60 #> log }
    route "/orders" { static "public"; handler list-orders #> auth #> retry times=3 }
}
"#;

    const EXAMPLE_NESTED: &str = r#"
app {
    route "/users" { log { cache ttl=(s)60 { auth role=admin { handler list-users } } } }
    route "/orders" { static "public"; retry times=3 { auth { handler list-orders } } }
}
"#;

    mod semantics {
        use super::*;

        #[test]
        fn a_single_pipe() {
            assert_equivalent(
                r#"text "Hello" #> padding 8"#,
                r#"padding 8 { text "Hello" }"#,
            );
        }

        #[test]
        fn chains_are_left_associative() {
            assert_equivalent("a #> b #> c", "c { b { a; }; }");
            assert_equivalent(EXAMPLE_PIPE, EXAMPLE_NESTED);
        }

        #[test]
        fn the_left_hand_side_can_have_children() {
            assert_equivalent(
                "sprite hero { frame 1; frame 2 } #> loop 4",
                "loop 4 { sprite hero { frame 1; frame 2 } }",
            );
        }

        #[test]
        fn the_right_hand_side_cannot_have_children() {
            assert_invalid(r#"text "Hello" #> padding 8 { border }"#);
        }

        #[test]
        fn tags_are_kept_on_every_stage() {
            assert_equivalent("(t)a 1 #> (u)b 2", "(u)b 2 { (t)a 1 }");
        }

        #[test]
        fn pipes_can_be_used_in_children() {
            assert_equivalent("x { a #> b; c }", "x { b { a }; c }");
        }
    }

    mod token {
        use super::*;

        #[test]
        fn whitespace_around_the_pipe_is_optional() {
            assert_equivalent("a#>b", "b { a }");
            assert_equivalent("a #>b", "b { a }");
            assert_equivalent("a#> b", "b { a }");
            assert_equivalent("a 1#>b", "b { a 1 }");
            assert_equivalent(r##"a "x"#>b"##, r#"b { a "x" }"#);
            assert_equivalent("r#>b", "b { r }");
        }

        #[test]
        fn the_pipe_is_not_a_value() {
            assert_invalid("a k=#> b");
            assert_invalid("a (#>)b");
        }
    }

    mod multiline {
        use super::*;

        #[test]
        fn trailing_pipe() {
            assert_equivalent(
                "source \"orders.csv\" #>\n\tfilter status=paid #>\n\tsort total",
                r#"sort total { filter status=paid { source "orders.csv" } }"#,
            );
        }

        #[test]
        fn leading_pipe() {
            assert_equivalent(
                "source \"orders.csv\"\n\t#> filter status=paid\n\t#> sort total\nnext",
                r#"sort total { filter status=paid { source "orders.csv" } }; next"#,
            );
        }

        #[test]
        fn comments_and_blank_lines_between_stages() {
            assert_equivalent(
                "source \"orders.csv\" // comment\n\n\t/* more */ #> filter status=paid\n\n\t#> sort total",
                r#"sort total { filter status=paid { source "orders.csv" } }"#,
            );
            assert_equivalent("a #> // comment\n\tb", "b { a }");
        }

        #[test]
        fn leading_pipe_continues_the_preceding_node_even_if_slashdashed() {
            assert_equivalent("a\n/- b\n#> c", "a");
            assert_equivalent("a\n/- b\n#> c\nd", "a; d");
        }

        #[test]
        fn leading_pipe_in_children() {
            assert_equivalent("x {\n\ta\n\t#> b\n\tc\n}", "x {\n\tb { a }\n\tc\n}");
        }

        #[test]
        fn a_semicolon_ends_the_chain() {
            assert_invalid("a; #> b");
            assert_invalid("a;\n#> b");
            assert_invalid("/- a;\n#> b");
        }

        #[test]
        fn the_pipe_requires_a_node_before_it() {
            assert_invalid("#> b");
            assert_invalid("\n\t#> b");
            assert_invalid("x { #> b }");
            assert_invalid("a #>");
            assert_invalid("a #> #> b");
        }

        #[test]
        fn parsing_a_single_node() {
            let nested = KdlNode::parse("b { a }").unwrap();
            for text in ["a #> b", "a\n#> b\n"] {
                let mut node = KdlNode::parse(text).unwrap_or_else(|e| panic!("{text:?}: {e:?}"));
                node.clear_format_recursive();
                let mut expected = nested.clone();
                expected.clear_format_recursive();
                assert_eq!(node, expected, "{text:?}");
            }
        }
    }

    mod slashdash {
        use super::*;

        #[test]
        fn slashdash_before_the_chain_drops_the_chain() {
            assert_equivalent(
                "/- image \"photo.jpg\" #> resize width=800 #> grayscale\nnext",
                "next",
            );
            assert_equivalent(
                "/- image \"photo.jpg\"\n#> resize width=800\n#> grayscale\nnext",
                "next",
            );
        }

        #[test]
        fn slashdash_before_the_pipe_drops_a_stage() {
            let nested = r#"center { text "Hello" }"#;
            assert_equivalent(r#"text "Hello" /-#> padding 8 #> center"#, nested);
            assert_equivalent(r#"text "Hello" /- #> padding 8 #> center"#, nested);
            assert_equivalent("text \"Hello\"\n\t/-#> padding 8\n\t#> center", nested);
            assert_equivalent(
                "sprite hero { frame 1 } /-#> loop 4 #> delay 2",
                "delay 2 { sprite hero { frame 1 } }",
            );
        }

        #[test]
        fn slashdash_on_the_final_stage() {
            assert_equivalent("a #> b /-#> c", "b { a }");
            assert_equivalent("a #> b\n/-#> c\nd", "b { a }; d");
            assert_equivalent("a /-#> c", "a");
        }

        #[test]
        fn slashdash_before_the_chain_across_lines() {
            assert_equivalent(
                "/- image \"photo.jpg\"\n\t#> resize width=800\n\n\t// note\n\t#> crop ratio=\"16/9\"\n\t#> grayscale\nnext",
                "next",
            );
            assert_equivalent("/- a #>\n\tb #> c\nnext", "next");
            assert_equivalent("/- a { x\n\ty } #> c\nnext", "next");
            assert_equivalent("/- a \"x;y\" #> b; d", "d");
            assert_equivalent("x {\n\t/- a\n\t#> b\n}", "x {}");
        }

        #[test]
        fn trailing_slashdashed_pipe_drops_the_next_stage() {
            assert_equivalent(
                "source \"orders.csv\" #>\n\tfilter status=paid /-#>\n\tgroup-by customer #>\n\tsort total order=desc",
                "sort total order=desc { filter status=paid { source \"orders.csv\" } }",
            );
            assert_equivalent("a /-#>   \n\tb\nc", "a; c");
            assert_equivalent("a /-#>\n\n\tb\nc", "a; c");
        }

        #[test]
        fn slashdashed_stage_ends_at_the_end_of_the_node() {
            assert_equivalent("a #> b /-#> c; d", "b { a }; d");
            assert_equivalent("x { a /-#> b }", "x { a }");
        }
    }

    mod formatting {
        use super::*;

        #[test]
        fn round_trip() {
            assert_round_trip(EXAMPLE_PIPE);
            assert_round_trip(r##"text "Hello"#>padding 8"##);
            assert_round_trip("a { b } #> c");
            assert_round_trip("a{b}#>c");
            assert_round_trip("(t)a 1 #> (u)b 2");
            assert_round_trip("source \"orders.csv\" #>\n\tfilter status=paid #>\n\tsort total\n");
            assert_round_trip(
                "source \"orders.csv\"\n\t#> filter status=paid // comment\n\n\t#> sort total\n",
            );
            assert_round_trip("/- image \"photo.jpg\" #> grayscale\nnext\n");
            assert_round_trip(r#"text "Hello" /-#> padding 8 #> center"#);
            assert_round_trip("text \"Hello\"\n\t/-#> padding 8\n\t#> center\n");
            assert_round_trip("a #> b /-#> c\n");
            assert_round_trip("a #> b\n/-#> c\nd\n");
            assert_round_trip("x {\n\ta\n\t#> b\n\tc\n}\n");
        }

        #[test]
        fn editing_the_piped_node_in_place_keeps_the_pipe() {
            let mut document = parse("a #> b");
            first_child(&mut document)
                .entries_mut()
                .push(KdlEntry::new(1));
            assert_eq!(document.to_string(), "a 1 #> b");
        }

        #[test]
        fn editing_a_node_in_the_middle_of_a_chain_keeps_the_pipe() {
            let mut document = parse("a #> b #> c");
            first_child(&mut document)
                .entries_mut()
                .push(KdlEntry::new(1));
            assert_eq!(document.to_string(), "a #> b 1 #> c");
        }

        #[test]
        fn adding_a_child_falls_back_to_the_nested_form() {
            let mut document = parse("a #> b");
            document.nodes_mut()[0]
                .ensure_children()
                .nodes_mut()
                .push(KdlNode::new("c"));
            assert_formats_nested(&document, "b { a; c }");
        }

        #[test]
        fn replacing_the_child_falls_back_to_the_nested_form() {
            let mut document = parse("a #> b");
            *first_child(&mut document) = KdlNode::new("z");
            assert_formats_nested(&document, "b { z }");
        }

        #[test]
        fn removing_the_child_falls_back_to_the_nested_form() {
            let mut document = parse("a #> b");
            document.nodes_mut()[0]
                .ensure_children()
                .nodes_mut()
                .clear();
            assert_formats_nested(&document, "b {}");
        }

        #[test]
        fn clear_format_and_autoformat_yield_the_nested_form() {
            // Call autoformat after clear_format_recursive to produce valid KDL.
            let mut cleared = parse(EXAMPLE_PIPE);
            cleared.clear_format_recursive();
            assert!(!cleared.to_string().contains("#>"));
            cleared.autoformat();
            assert_formats_nested(&cleared, EXAMPLE_NESTED);

            let mut autoformatted = parse(EXAMPLE_PIPE);
            autoformatted.autoformat();
            assert_formats_nested(&autoformatted, EXAMPLE_NESTED);
        }

        #[test]
        fn clone_keeps_the_pipe() {
            assert_eq!(parse(EXAMPLE_PIPE).clone().to_string(), EXAMPLE_PIPE);
        }
    }

    #[cfg(feature = "span")]
    #[test]
    fn spans_only_cover_the_node_itself() {
        let document = parse(r#"text "Hello" #> padding 8"#);
        let padding = &document.nodes()[0];
        let text = &padding.children().as_ref().expect("children").nodes()[0];

        assert_eq!(padding.span().offset(), 16);
        assert_eq!(text.span().offset(), 0);
        assert!(text.span().offset() + text.span().len() <= 13);
    }
}
