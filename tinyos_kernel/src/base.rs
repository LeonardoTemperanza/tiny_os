
// This will contain some utility functions
// that are probably part of the standard library,
// which we can't use.

pub struct Reader<'a>
{
    pub buf: &'a [u8],
    pub offset: usize,
}

impl<'a> Reader<'a>
{
    pub fn new(buf: &'a [u8])->Self
    {
        return Reader { buf: buf, offset: 0 };
    }

    pub fn read<T>(&mut self, data: &mut T)
    {
        let data_size = core::mem::size_of::<T>();
        if self.offset + data_size > self.buf.len() {
            panic!("Buffer overflow detected in reader.");
        }

        unsafe
        {
            let data_ptr = data as *mut T as *mut u8;
            let buf_ptr = (&self.buf[self.offset]) as *const u8;
            copy_unaligned(buf_ptr, data_ptr, data_size);
        }

        self.offset += data_size;
    }

    pub fn read_len_string(&mut self, string: &mut &'a str, num_bytes: usize)
    {
        if self.offset + num_bytes > self.buf.len() {
            panic!("Buffer overflow detected in reader.");
        }

        let slice = &self.buf[self.offset..self.offset + num_bytes];
        // TODO: Doesn't necessarily have to be valid UTF8
        *string = core::str::from_utf8(&slice[..]).expect("Not valid UTF8");

        self.offset += num_bytes;
    }
}

unsafe fn copy_unaligned(src: *const u8, dst: *mut u8, count: usize)
{
    let src = src as *const u8;
    let dst = dst as *mut u8;

    for i in 0..count {
        unsafe { core::ptr::write(dst.add(i), core::ptr::read(src.add(i))) };
    }
}
