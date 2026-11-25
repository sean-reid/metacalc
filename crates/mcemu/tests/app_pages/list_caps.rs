use crate::table;
use mcui::fold::{fit_one, layout};
use mcui::{Installed, Status};

#[test]
fn explore_caps() {
    let mut extra = String::from("wsub $a wff A ph $.\n");
    for (name, lead, k) in [
        ("n126", "", 126),
        ("n127", "", 127),
        ("n128", "", 128),
        ("m127", "-. ", 127),
        ("m126", "-. ", 126),
        ("m130", "-. ", 130),
    ] {
        extra += &format!("{name} $a |- {lead}{}ph $.\n", "A ".repeat(k));
    }
    let library = crate::fixture::custom(&extra);
    let installed = Installed {
        name: "library".into(),
        bytes: library.clone(),
    };
    let n = usize::from(table::header_of(&library).1);
    let lib = mcui::data::Lib::new(installed, vec![Status::Unchecked; n]).expect("the model loads");
    for i in 0..n {
        let c = &lib.statement(i).conclusion;
        if c.tokens().len() > 100 || i + 8 > n {
            println!(
                "{i}: tokens {} nodes {} within {}",
                c.tokens().len(),
                c.len(),
                mcui::caps::statement_within(c)
            );
        }
    }
    let _ = (fit_one, layout);
}
