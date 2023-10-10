use std::process::Command;
use std::time::{Duration, SystemTime};
use std::{fs, thread};
fn main() {
    let mut last_content = get_clipboard_content();
    loop {
        let content = get_clipboard_content();
        if content.len() != last_content.len() || !content.eq(&last_content) {
            println!("Has changed: {:?}", content);
            // {{timestamp}}.txt
            let filename = format!(
                "{}.png",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis()
            );
            fs::write(filename, &content).expect("Unable to write file");
            last_content = content;
        }
        thread::sleep(Duration::from_millis(200));
    }
}
// Using xclip
fn get_clipboard_content() -> Vec<u8> {
    //xclip -selection clipboard -o
    // xclip -selection clipboard -o -t image/png
    let content = Command::new("xclip")
        .arg("-o")
        .arg("-selection")
        .arg("clipboard")
        .arg("-t")
        .arg("image/png")
        .output()
        .expect("failed to execute process");
    content.stdout
}
