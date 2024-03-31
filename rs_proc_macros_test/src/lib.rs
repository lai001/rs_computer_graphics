use rs_proc_macros::MultipleThreadFunctionsGenerator;
use std::sync::{Arc, Mutex};

struct STResourceManager {}

impl STResourceManager {
    fn new() -> STResourceManager {
        STResourceManager {}
    }

    fn test(&self) -> i32 {
        100
    }

    fn test2(&self) {
        println!("test2")
    }

    fn test3(&self, v1: i32, v2: f32) -> f32 {
        v1 as f32 + v2
    }

    fn test4<T: AsRef<str>>(&self, v1: T) -> String {
        v1.as_ref().to_string()
    }
}

#[derive(Clone, MultipleThreadFunctionsGenerator)]
#[file("rs_proc_macros_test/src/lib.rs", "STResourceManager")]
#[ignore_functions("new")]
pub struct ResourceManager {
    inner: Arc<Mutex<STResourceManager>>,
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        ResourceManager {
            inner: Arc::new(Mutex::new(STResourceManager::new())),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ResourceManager;

    #[test]
    fn test() {
        let resource_manager = ResourceManager::new();
        assert_eq!(resource_manager.test(), 100);
        resource_manager.test2();
        assert_eq!(resource_manager.test3(1, 1.0), 2.0);
        assert_eq!(resource_manager.test4("abc"), "abc");
    }
}
