use std::env;
use std::io::{self,BufRead,Write};
use std::collections::HashMap;

extern crate regex;
use regex::Regex;

#[derive(Debug)]
enum Parsed {
    Str(String),
    Tree(Vec<Parsed>),
}

fn parse(s: &Vec<char>, mut i: usize, lvl: usize) -> (usize, Parsed) {
    let mut buf: Vec<char> = Vec::new();
    let mut tree: Vec<Parsed> = Vec::new();
    let mut escape = false;

    while i < s.len() {
        let c = s[i];

        if escape {
            if i == s.len() - 1 {
                buf.push('\\');
                escape = false;
            } else {
                if "{}".contains(c) {
                    buf.push(c);
                } else if c == '\\' {
                    // Rust regex eats one '\'. Supply two to compensate
                    buf.push(c);
                    buf.push(c);
                } else {
                    buf.push('\\');
                    buf.push(c);
                }
                i += 1;
                escape = false;
                continue
            }
        }

        if c == '\\' {
            i += 1;
            escape = true;
            continue
        }

        if c == '{' {
            tree.push(Parsed::Str(buf.iter().collect()));
            let (i_, subtree) = parse(s, i + 1, lvl + 1);
            i = i_;
            tree.push(subtree);
            buf.clear();
        } else if c == '}' {
            tree.push(Parsed::Str(buf.iter().collect()));
            if lvl == 0 && i < s.len() - 1 {
                //raise ValueError("Parse error: closing }")
            }
            return (i, Parsed::Tree(tree))
        } else {
            buf.push(c);
        }

        i += 1;
    }

    (i, Parsed::Tree(tree))
}

fn to_regex_(tree: &[Parsed], is_group: bool, result: &mut Vec<String>) {
    if is_group {
        let first = match tree[0] {Parsed::Str(ref s) => s, _ => {panic!("unreachable")} };

        let (group, rx) = if first.len() == 0 {
            ("EEE", r"[^/]*")
        } else if let Some(i) = first.find(':') {
            (&first[..i], &first[i + 1..])
        } else {
            let rx = if first.chars().all(|c| c.is_ascii_lowercase()) {
                r"[^/]*"
            } else if first.chars().all(|c| c.is_ascii_alphabetic()) {
                r"\w*"
            } else if first.chars().all(|c| c.is_ascii_digit()) {
                r"\d*"
            } else {
                r"[^/]*"
            };
            (first.as_str(), rx)
        };

        result.push(format!("(?P<P{}>{}", group, rx));
        to_regex_(&tree[1..], false, result);
        result.push(String::from(")"));
    } else {
        for x in tree {
            match x {
                Parsed::Str(s) => result.push(s.to_string()),
                Parsed::Tree(ref subtree) => to_regex_(subtree, true, result),
            }
        }
    }
}

fn to_regex(tree: &Parsed) -> String {
    let mut result: Vec<String> = Vec::new();
    if let &Parsed::Tree(ref tree1) = tree {
        if let Parsed::Tree(ref tree2) = tree1[1] {
            to_regex_(tree2, false, &mut result);
        }
    }
    result.join("")
}

fn assemble(s: &[&str], dict: &HashMap<&str, &str>) -> String {
    let mut res: Vec<char> = s[0].chars().collect();
    for i in (1..s.len()).step_by(2) {
        let key = format!("P{}", s[i]);
        if let Some(m) = dict.get(key.as_str()) {
            res.extend(m.chars());
        }
        res.extend(s[i + 1].chars());
    }
    res.iter().collect()
}

fn main() {
    let mut pats_: Vec<&str> = Vec::new();
    let mut subs_: Vec<&str> = Vec::new();

    let args: Vec<String> = env::args().collect();
    for i in (1..args.len()).step_by(2) {
        pats_.push(args[i].as_ref());
        if i + 1 == args.len() {
            subs_.push(args[i].as_ref());
        } else {
            subs_.push(args[i + 1].as_ref());
        }
    }

    let pats: Vec<Regex> = pats_.iter().map(|arg| {
        let pat = format!("{{{}}}", arg);
        let (_, tree) = parse(&pat.chars().collect(), 0, 0);
        println!("tree {:?}", tree);
        let rx = format!("({})", to_regex(&tree));
        println!("rx {:?}", rx);

        match Regex::new(&rx) {
            Result::Ok(x) => x,
            Result::Err(e) => {
                println!("Couldn't parse regex pattern '{}': {}", &pat[1..pat.len() - 1], e);
                std::process::exit(1);
            },
        }
    }).collect();

    //println!("REG {:?}", pats);

    let subs: Vec<Vec<&str>> = subs_.iter().map(|s| s.split(&['{', '}'][..]).collect()).collect();

    for line in io::stdin().lock().lines() {
        // safe to unwrap. Docs do it
        let x = line.unwrap();

        for (pat, sub) in pats.iter().by_ref().zip(subs.iter()) {
            if let Some(captures) = pat.captures(x.as_str()) {
                // There is always at least one capture, because pattern was wrapped in ()
                // unwrap is safe
                let main_capture = captures.iter().next().unwrap().unwrap();
                let mut dict: HashMap<&str, &str> =
                    pat
                    .capture_names()
                    .flatten()
                    .filter_map(|name| Some((name, captures.name(name)?.as_str())))
                    .collect();
                dict.insert("P@", &x);
                dict.insert("P%", &x[main_capture.start()..main_capture.end()]);
                dict.insert("P^", &x[..main_capture.start()]);
                dict.insert("P$", &x[main_capture.end()..]);

                let y = assemble(&sub[..], &dict);
                let _ = io::stdout().write_all(y.as_bytes());
                let _ = io::stdout().write_all(b"\n");
                let _ = io::stdout().flush();
            }
        }
    }
}
