use std::env;
//use std::io::{self,BufRead,Write};

//extern crate regex;
//use regex::Regex;

#[derive(Debug)]
enum Parsed {
    Base(String),
    Tree(Vec<Parsed>),
}

fn parse(s: &Vec<char>, mut i: usize, lvl: usize) -> (usize, Parsed) {
    let mut buf: Vec<char> = Vec::new();
    let mut tree: Vec<Parsed> = Vec::new();

    while i < s.len() {
        if s[i] == '{' {
            tree.push(Parsed::Base(buf.iter().collect()));
            let (j, subtree) = parse(s, i + 1, lvl + 1);
            i = j;
            tree.push(subtree);
            buf.clear();
        } else if s[i] == '}' {
            tree.push(Parsed::Base(buf.iter().collect()));
            if lvl == 0 && i < s.len() - 1 {
                //raise ValueError("Parse error: closing }")
            }
            return (i, Parsed::Tree(tree))
        } else {
            buf.push(s[i]);
        }

        i += 1;
    }

    (i, Parsed::Tree(tree))
}

fn to_regex_(tree: &[Parsed], is_group: bool, result: &mut Vec<String>) {
    if is_group {
        let name = match tree[0] {Parsed::Base(ref s) => s, _ => {panic!("unreachable")} };
        let rx = "[^/]*";
        result.push(format!("(?P<{}>{}", name, rx));

        to_regex_(&tree[1..], false, result);
        result.push(String::from(")"));
    } else {
        for x in tree {
            match x {
                Parsed::Base(s) => result.push(s.to_string()),
                Parsed::Tree(ref subtree) => to_regex_(subtree, true, result),
            }
        }
    }
}

fn to_regex(tree: &Parsed) -> String {
    let mut result: Vec<String> = Vec::new();
    if let &Parsed::Tree(ref tree1) = tree {
        if let Parsed::Tree(ref tree2) = tree1[1] {
            println!("yyy {:?}", tree2);
            to_regex_(tree2, false, &mut result);
        }
    }
    result.join("")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("argz {:?}", args);

    let pat = format!("{{{}}}", &args[1]);
    let (_, tree) = parse(&pat.chars().collect(), 0, 0);
    let rx = to_regex(&tree);
    println!("rx {}", rx);
//
//    let mut pats_: Vec<&str> = Vec::new();
//    let mut subs_: Vec<&str> = Vec::new();
//
//    for i in (1..args.len()).step_by(2) {
//        pats_.push(args[i].as_ref());
//        if i + 1 == args.len() {
//            subs_.push(args[i].as_ref());
//        } else {
//            subs_.push(args[i + 1].as_ref());
//        }
//    }
//    println!("{:?}", pats_);
//    println!("{:?}", subs_);
//    let res: Vec<Regex> = args.iter().map(|x| Regex::new(x.as_str()).unwrap()).collect();
//
//    for line in io::stdin().lock().lines() {
//        let s = line.unwrap();
//        for (re, _) in res.iter().by_ref().zip(res.iter().by_ref()) {
//            if let Some(m) = re.find(s.as_str()) {
//                let _ = io::stdout().write_all(s.as_bytes());
//                let _ = io::stdout().write_all(b"\n");
//                let _ = io::stdout().flush();
//            }
//        }
//    }
}
