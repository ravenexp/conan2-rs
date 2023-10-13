use std::ffi::{c_int, c_uint, c_ulong, c_void};

extern "C" {
    /// zlib
    fn adler32(adler: c_ulong, buf: *const c_void, len: c_uint) -> c_ulong;

    /// libxml2
    fn xmlCheckVersion(version: c_int);

    /// libiconv
    fn iconv_open(tocode: *const u8, fromcode: *const u8) -> *const c_void;

    /// libcharset
    fn locale_charset() -> *const u8;

    /// libcrypto
    fn EVP_MD_CTX_new() -> *const c_void;

    /// libssl
    fn OPENSSL_init_ssl(opts: u64, buf: *const c_void) -> c_int;
}

fn main() {
    unsafe {
        // zlib
        adler32(0, std::ptr::null(), 0);

        // libxml2
        xmlCheckVersion(2_00_00);

        // libiconv
        iconv_open(b"a".as_ptr(), b"b".as_ptr());

        // libcharset
        locale_charset();

        // libcrypto
        EVP_MD_CTX_new();

        // libssl
        OPENSSL_init_ssl(0, std::ptr::null());
    };
}
