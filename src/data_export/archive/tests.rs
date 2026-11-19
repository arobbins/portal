use super::*;

#[test]
fn values_a_spreadsheet_would_run_get_a_leading_apostrophe() {
    for risky in ["=1+1", "+cmd", "-2+3", "@SUM(1,2)", "\tx", "\rx", "-", "+"] {
        assert_eq!(spreadsheet_safe(risky), format!("'{risky}"), "{risky:?}");
    }
}

#[test]
fn plain_numbers_and_ordinary_text_stay_as_they_are() {
    for safe in [
        "-5",
        "+3.2",
        "42",
        "-0.5",
        "Ann",
        "",
        "2026-11-18T14:03:00Z",
        "a=b",
    ] {
        assert_eq!(spreadsheet_safe(safe), safe, "{safe:?}");
    }
}

#[test]
fn every_cell_of_every_file_is_protected_and_the_readme_says_so() {
    let dataset = Dataset {
        file: "profile.csv",
        about: "About.",
        tables: &["users"],
        columns: &["name", "amount"],
        rows: vec![vec!["=1+1".to_string(), "-5".to_string()]],
    };

    let csv = String::from_utf8(csv(&dataset).unwrap()).unwrap();

    assert_eq!(csv, "name,amount\n'=1+1,-5\n");
    assert!(readme(&[dataset]).contains("remove that apostrophe to get the original value"));
}
