use std::path::PathBuf;

use super::{FrameRequest, FrameSource};

pub struct FrameChain {
    sources: Vec<Box<dyn FrameSource>>,
}

impl FrameChain {
    pub fn new(sources: Vec<Box<dyn FrameSource>>) -> Self {
        Self { sources }
    }
}

impl FrameSource for FrameChain {
    fn render(&self, request: &FrameRequest) -> Option<PathBuf> {
        self.sources.iter().find_map(|s| s.render(request))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    struct Counted(Option<&'static str>, Arc<AtomicUsize>);

    impl FrameSource for Counted {
        fn render(&self, _: &FrameRequest) -> Option<PathBuf> {
            self.1.fetch_add(1, Ordering::SeqCst);
            self.0.map(PathBuf::from)
        }
    }

    #[test]
    fn stops_at_the_first_source_that_answers() {
        let calls = Arc::new(AtomicUsize::new(0));
        let chain = FrameChain::new(vec![
            Box::new(Counted(None, calls.clone())),
            Box::new(Counted(Some("b.png"), calls.clone())),
            Box::new(Counted(Some("c.png"), calls.clone())),
        ]);
        let req = FrameRequest {
            video: "v.mp4".into(),
            out: "o.png".into(),
            size: 64,
        };
        assert_eq!(chain.render(&req), Some(PathBuf::from("b.png")));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(FrameChain::new(Vec::new()).render(&req), None);
    }
}
