fn main() { let ds = wasm_encoder::DataSegment::active(0, wasm_encoder::ConstExpr::i32_const(0), &[1, 2, 3]); println!("{:?}", ds); }
