fn main() {
    let args = appleargs::apple_args();
    let argc = args.len();
    if argc == 0 {
        println!("No apple arguments found (unsupported target?)");
        return;
    }
    println!("{} apple arguments given to this process:", args.len());
    for (i, entry) in args.enumerate() {
        println!("[{i}]: {entry:?}");
    }
}
