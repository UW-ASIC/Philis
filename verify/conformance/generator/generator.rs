mod helper;
mod drc_gen;
mod lvs_gen;
mod pex_gen;
mod erc_gen;

use helper::Suite;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "conformance".to_string());

    let mut s = Suite::new();
    drc_gen::generate(&mut s);
    lvs_gen::generate(&mut s);
    pex_gen::generate(&mut s);
    erc_gen::generate(&mut s);
    s.gds.end_lib();

    std::fs::create_dir_all(&dir).expect("create output dir");

    let gds = s.gds.finish();
    std::fs::write(format!("{dir}/conformance.gds"), &gds).expect("write gds");

    let manifest = serde_json::to_string_pretty(&s.manifest()).expect("serialize manifest");
    std::fs::write(format!("{dir}/manifest.json"), &manifest).expect("write manifest");

    let params = serde_json::to_string_pretty(&helper::params_json()).expect("serialize params");
    std::fs::write(format!("{dir}/params.json"), &params).expect("write params");

    println!("generated {} bytes GDS, {} DRC + {} LVS + {} PEX + {} ERC cases -> {dir}/",
        gds.len(), s.drc_cases.len(), s.lvs_cases.len(), s.pex_cases.len(), s.erc_cases.len());
}
