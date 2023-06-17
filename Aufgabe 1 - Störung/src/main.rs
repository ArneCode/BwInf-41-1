use std::{env, fs, io::stdin};

fn find_pattern(text: &str, searched: &str) -> Vec<(usize, String)> {
    //text und searched wird in Vec<char> umgewandelt
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let searched: Vec<char> = searched.to_lowercase().chars().collect();
    let mut results = vec![];
    let mut line_n = 1;
    'text_loop: for text_i in 0..text.len() {
        if text[text_i] == '\n' {
            line_n += 1;
        }
        let mut offset = 0;
        'searched_loop: for searched_i in 0..searched.len() {
            if let Some(c) = text.get(text_i + searched_i + offset) {
                //
                match searched[searched_i] {
                    '_' => {
                        //ein wort überspringen
                        while let Some(c) = text.get(text_i + searched_i + offset) {
                            if c.is_whitespace() {
                                //ende des Wortes
                                offset -= 1; //whitespace ausschließen
                                break;
                            }
                            offset += 1;
                        }
                        continue 'searched_loop;
                    }
                    searched_c => {
                        if c == &searched_c {
                            continue 'searched_loop;
                        }
                    }
                }
            }
            //die Stelle passt nicht zum Lückensatz
            continue 'text_loop;
        }
        //Die Stelle passt zum Lückensatz
        let result_str = String::from_iter(&text[text_i..text_i + searched.len() + offset]);
        results.push((line_n, result_str));
    }
    results
}
fn main() {
    let whole_text = fs::read_to_string("data/Alice_im_Wunderland.txt")
        .expect("konnte data/Alice_im_Wunderland.txt nicht finden");
    //Die Programmargumente
    let args: Vec<String> = env::args().collect();
    let searched_snippet = match args.get(1) {
        Some(path) => {
            fs::read_to_string(path).expect(&format!("konnte Datei nicht auslesen: {}", path))
        }
        None => {
            //Kein Programmargument
            let mut inp = String::new();
            println!("bitte den gesuchten Lückensatz eingeben: ");
            stdin()
                .read_line(&mut inp)
                .expect("konnte Input nicht auslesen");
            inp.trim().to_string()
        }
    };
    println!("gesucht: '{}'", searched_snippet);
    let result = find_pattern(&whole_text, &searched_snippet);
    for (line_n, result_str) in &result {
        println!(
            "Passende Stelle in Zeile {} gefunden: {}",
            line_n, result_str
        );
    }
    if result.is_empty() {
        println!("keine Passende Stelle für '{}' gefunden!", searched_snippet);
    }
}
