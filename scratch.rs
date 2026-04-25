fn refine_model_name(brand: &str, model: &str) -> String {
    let noise = ["ASUSTEK", "COMPUTER", "INC", "CORP", "CORPORATION", "LTD", "SYSTEMS", "PRODUCT", "NAME", "LAPTOP"];
    
    // 2. Limpieza inicial de caracteres y normalización
    let mut clean = model
        .replace("_", " ")
        .replace("ASUSLaptop", " Laptop ")
        .replace("-", " ")
        .trim()
        .to_string();

    // Eliminar dobles espacios
    while clean.contains("  ") {
        clean = clean.replace("  ", " ");
    }

    let words: Vec<&str> = clean.split_whitespace().collect();
    let mut unique_words: Vec<String> = Vec::new();
    let brand_up = brand.to_uppercase();

    for &word in &words {
        let word_up = word.to_uppercase().replace(".", "");
        
        if noise.contains(&word_up.as_str()) {
            continue;
        }

        if word_up == brand_up || word_up.contains(&brand_up) {
            continue;
        }

        if let Some(last) = unique_words.last() {
            let last_up = last.to_uppercase();
            if word_up == last_up || word_up.starts_with(&last_up) || last_up.starts_with(&word_up) {
                if word.len() > last.len() {
                    unique_words.pop();
                    unique_words.push(word.to_string());
                }
                continue;
            }
        }
        
        unique_words.push(word.to_string());
    }

    let result_model = unique_words.join(" ");
    
    if result_model.is_empty() {
        return brand.to_string();
    }

    format!("{} {}", brand, result_model)
}

fn main() {
    let test_cases = vec![
        ("ASUS", "Vivobook_ ASUSLaptop X1404VA_X1404VA"),
        ("ASUS", "Vivobook_ASUSLaptop M1502YA_M1502YA"),
        ("ASUS", "ASUS Vivobook Go E1404GAB_E1404GA"),
        ("ASUS", "ASUSTeK COMPUTER INC. ASUS TUF Gaming F16 FX607VJ_FX607VJ")
    ];

    for (brand, model) in test_cases {
        println!("Raw: '{}'\n=> Refined: '{}'\n", model, refine_model_name(brand, model));
    }
}
