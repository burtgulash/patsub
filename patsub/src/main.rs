use std::env;
use std::io::{self,BufRead,Write};

extern crate regex;
use regex::Regex;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    let mut pats: Vec<&str> = Vec::new();
    let mut subs: Vec<&str> = Vec::new();

    for i in (1..args.len()).step_by(2) {
        pats.push(args[i].as_ref());
        if i + 1 == args.len() {
            subs.push(args[i].as_ref());
        } else {
            subs.push(args[i + 1].as_ref());
        }
    }
    println!("{:?}", pats);
    println!("{:?}", subs);
    return;
    let res: Vec<Regex> = args.iter().map(|x| Regex::new(x.as_str()).unwrap()).collect();

    for line in io::stdin().lock().lines() {
        let s = line.unwrap();
        for re in &res {
            if let Some(m) = re.find(s.as_str()) {
                let _ = io::stdout().write_all(s.as_bytes());
                let _ = io::stdout().write_all(b"\n");
                let _ = io::stdout().flush();
            }
        }
    }
}
