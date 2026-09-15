use kobeat::daemon::decoder::symphonia_decoder::SymphoniaDecoder;

fn main() -> anyhow::Result<()> {
    let path = "/tmp/kobeat_test.mp3";
    let mut d = SymphoniaDecoder::open(path)?;
    println!("duration_secs = {:?}", d.duration_secs());
    println!("title         = {:?}", d.title());
    println!("sample_rate   = {}", d.sample_rate());
    println!("channels      = {}", d.channels());

    let mut total = 0usize;
    let mut out = Vec::new();
    while !d.done {
        d.read_pcm_s16le(&mut out)?;
        total += out.len();
    }
    println!("decoded bytes = {total}");

    let mut d2 = SymphoniaDecoder::open(path)?;
    d2.seek(1.5)?;
    let mut after = 0usize;
    let mut out2 = Vec::new();
    while !d2.done {
        d2.read_pcm_s16le(&mut out2)?;
        after += out2.len();
    }
    println!("after seek(1.5) bytes = {after}");
    Ok(())
}
