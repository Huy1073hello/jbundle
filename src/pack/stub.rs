pub fn generate(payload_hash: &str, payload_size: u64, jvm_args: &[String]) -> String {
    let jvm_args_str = if jvm_args.is_empty() {
        String::new()
    } else {
        format!(" {}", jvm_args.join(" "))
    };

    format!(
        r#"#!/bin/sh
set -e
CACHE_ID="{payload_hash}"
CACHE_DIR="${{HOME}}/.clj-pack/cache/${{CACHE_ID}}"
PAYLOAD_SIZE={payload_size}

if [ ! -d "$CACHE_DIR/runtime" ]; then
    mkdir -p "$CACHE_DIR"
    echo "Extracting runtime (first run)..." >&2
    tail -c "$PAYLOAD_SIZE" "$0" | tar xzf - -C "$CACHE_DIR"
fi

exec "$CACHE_DIR/runtime/bin/java"{jvm_args_str} -jar "$CACHE_DIR/app.jar" "$@"
exit 0
# --- PAYLOAD BELOW ---
"#
    )
}
