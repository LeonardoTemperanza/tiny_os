
// This will contain some utility functions
// that are probably part of the standard library,
// which we can't use.

use core::mem::{size_of, align_of};

pub struct BinaryParser<'a>
{
    pub buf: &'a [u8],
    pub offset: usize,
    pub do_align: bool,
}

impl<'a> BinaryParser<'a>
{
    pub fn new(buf: &'a [u8], offset: usize, do_align: bool)->Self
    {
        return BinaryParser {buf, offset, do_align};
    }

    pub fn next_str(self: &mut Self, num_chars: usize)->&'a [u8]
    {
        let res = &self.buf[self.offset..self.offset+num_chars];
        self.offset += num_chars;
        return res;
    }


    pub fn next<T>(self: &mut Self)->T
    {
        // Apply padding
        if self.do_align
        {
            self.offset = align_forward(self.buf, self.offset, align_of::<T>());
            let byte_ref = &self.buf[self.offset];
            let res = unsafe { &*(byte_ref as *const u8 as *const T) };
            self.offset += size_of::<T>();
            return *res;
        }
        else
        {
            
        }
    }
}

pub fn is_power_of_two(n: usize)->bool
{
    return (n & (n-1)) == 0;
}

pub fn align_forward(buf: &[u8], offset: usize, align: usize)->usize
{
    assert!(is_power_of_two(align));

    // This is the same as (ptr % a) but faster
    // since a is known to be a power of 2
    let ptr = buf.as_ptr() as usize + offset;
    let modulo = ptr & (align - 1);

    if modulo != 0
    {
        return offset + align - modulo;
    }
    else
    {
        return offset;
    }
}
