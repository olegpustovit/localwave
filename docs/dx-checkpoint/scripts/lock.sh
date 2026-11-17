# source me: take_lock  — mkdir lock with owner pid; breaks a lock whose owner is gone
LOCK=/private/tmp/claude-501/dx-macbench.lock
take_lock() { while ! mkdir "$LOCK" 2>/dev/null; do o=$(cat "$LOCK/pid" 2>/dev/null); if [ -n "$o" ] && ! kill -0 "$o" 2>/dev/null; then rm -rf "$LOCK"; continue; fi; if [ -z "$o" ] && [ $(( $(date +%s) - $(stat -f %m "$LOCK" 2>/dev/null || date +%s) )) -gt 120 ]; then rm -rf "$LOCK"; continue; fi; sleep 5; done; echo $$ > "$LOCK/pid"; }
