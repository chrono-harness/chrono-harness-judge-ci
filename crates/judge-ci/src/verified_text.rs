//! Exact comparison of UTF-8 text used by CI admission and preparation.
use vstd::prelude::*;

verus! {
pub fn equal_text(left: &str, right: &str) -> (equal: bool)
    ensures equal == (left@ == right@),
{
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    if left_bytes.len() != right_bytes.len() { return false; }
    let mut index = 0;
    while index < left_bytes.len()
        invariant
            left_bytes@ == vstd::utf8::encode_utf8(left@),
            right_bytes@ == vstd::utf8::encode_utf8(right@),
            index <= left_bytes.len() == right_bytes.len(),
            forall|prior: int| 0 <= prior < index ==> left_bytes@[prior] == right_bytes@[prior],
        decreases left_bytes.len() - index,
    {
        if left_bytes[index] != right_bytes[index] { return false; }
        index += 1;
    }
    proof {
        assert(left_bytes@ =~= right_bytes@);
        vstd::utf8::encode_utf8_decode_utf8(left@);
        vstd::utf8::encode_utf8_decode_utf8(right@);
    }
    true
}
}
