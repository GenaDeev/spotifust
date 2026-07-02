fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/spotifust.ico");
        res.compile().unwrap();
    }
}
