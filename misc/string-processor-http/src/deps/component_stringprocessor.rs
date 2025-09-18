wit_bindgen::generate!({
    inline: r#"
    package imported:component-stringprocessor;
    world imports {
        import component:stringprocessor/stringprocessing;
    }
    "#,
    with: {
        "component:stringprocessor/stringprocessing": generate,
    },
    path: ".wit/components/deps/stringprocessor/component-stringprocessor.wit",
});
