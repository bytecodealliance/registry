fn main() {
    let mut args = std::env::args();
    let arg0 = args.next().unwrap();
    let pattern = args.next().unwrap_or_else(|| {
        fatal(format!("usage: {arg0} <pattern>"));
    });

    for line_res in std::io::stdin().lines() {
        let line = line_res.unwrap_or_else(|err| fatal(err));
        if line.contains(&pattern) {
            #[cfg(feature = "evil")]
            let line: String = line.chars().rev().collect();

            println!("{line}");
        }
    }
}

fn fatal(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1)
}
