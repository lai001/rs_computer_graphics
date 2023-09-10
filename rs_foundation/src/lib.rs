pub mod id_generator;
pub mod profiler;

#[repr(C)]
#[derive(Clone, Default, PartialEq, Eq, Hash, Debug)]
pub struct Range<T: Copy> {
    pub start: T,
    pub end: T,
}

impl<T> Range<T>
where
    T: Copy,
{
    pub fn to_std_range(&self) -> std::ops::Range<T> {
        std::ops::Range::<T> {
            start: self.start,
            end: self.end,
        }
    }
}

#[derive(Debug)]
pub struct TimeRange {
    pub start: f32,
    pub end: f32,
}

impl TimeRange {
    pub fn is_contains(&self, time: f32) -> bool {
        time >= self.start && time <= self.end
    }
}

pub fn ffi_to_rs_string(c_str: *const std::ffi::c_char) -> Option<String> {
    if c_str.is_null() {
        None
    } else {
        let rs_string = unsafe { std::ffi::CStr::from_ptr(c_str).to_str().unwrap().to_owned() };
        Some(rs_string)
    }
}

pub fn math_remap_value_range(
    value: f64,
    from_range: std::ops::Range<f64>,
    to_range: std::ops::Range<f64>,
) -> f64 {
    (value - from_range.start) / (from_range.end - from_range.start)
        * (to_range.end - to_range.start)
        + to_range.start
}

pub fn get_object_address<T>(object: &T) -> String {
    let raw_ptr = object as *const T;
    std::format!("{:?}", raw_ptr)
}

pub fn cast_to_raw_buffer<'a, T>(vec: &[T]) -> &'a [u8] {
    let buffer = vec.as_ptr() as *const u8;
    let size = std::mem::size_of::<T>() * vec.len();
    let buffer = unsafe { std::slice::from_raw_parts(buffer, size) };
    buffer
}

pub fn cast_to_raw_type_buffer<'a, U>(buffer: *const u8, len: usize) -> &'a [U] {
    unsafe {
        let len = len / std::mem::size_of::<U>();
        std::slice::from_raw_parts(buffer as *const U, len)
    }
}

pub fn cast_to_type_buffer<'a, U>(buffer: &'a [u8]) -> &'a [U] {
    unsafe {
        let len = buffer.len() / std::mem::size_of::<U>();
        std::slice::from_raw_parts(buffer.as_ptr() as *const U, len)
    }
}

pub fn alignment(n: isize, align: isize) -> isize {
    return ((n) + (align) - 1) & !((align) - 1);
}

pub fn next_highest_power_of_two(v: isize) -> isize {
    let mut v = v;
    v = v - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v = v + 1;
    v
}

#[cfg(test)]
pub mod test {
    use crate::{alignment, math_remap_value_range, next_highest_power_of_two};

    #[test]
    pub fn next_highest_power_of_two_test() {
        assert_eq!(next_highest_power_of_two(418), 512);
    }

    #[test]
    pub fn alignment_test() {
        assert_eq!(alignment(418, 4), 420);
    }

    #[test]
    pub fn math_remap_value_range_test() {
        let mapped_value = math_remap_value_range(
            1.0,
            std::ops::Range::<f64> {
                start: 0.0,
                end: 2.0,
            },
            std::ops::Range::<f64> {
                start: 0.0,
                end: 100.0,
            },
        );
        assert_eq!(mapped_value, 50.0_f64);

        let mapped_value = math_remap_value_range(
            0.0,
            std::ops::Range::<f64> {
                start: 0.0,
                end: 2.0,
            },
            std::ops::Range::<f64> {
                start: 0.0,
                end: 100.0,
            },
        );
        assert_eq!(mapped_value, 0.0_f64);

        let mapped_value = math_remap_value_range(
            2.0,
            std::ops::Range::<f64> {
                start: 0.0,
                end: 2.0,
            },
            std::ops::Range::<f64> {
                start: 0.0,
                end: 100.0,
            },
        );
        assert_eq!(mapped_value, 100.0_f64);

        let mapped_value = math_remap_value_range(
            -1.0,
            std::ops::Range::<f64> {
                start: 0.0,
                end: 2.0,
            },
            std::ops::Range::<f64> {
                start: 0.0,
                end: 100.0,
            },
        );
        assert_eq!(mapped_value, -50.0_f64);
    }
}
