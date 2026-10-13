portaki_sdk::portaki_module!(id = "refusals", error = "photo_required");

fn main() {
    let _ = portaki_sdk::refuse!("host_only");
}
