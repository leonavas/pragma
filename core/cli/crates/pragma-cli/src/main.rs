fn main() {
    std::process::exit(pragma_cli::run(std::env::args().skip(1).collect()));
}
