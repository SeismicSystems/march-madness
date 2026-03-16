# March Madness deploy aliases — source from ~/.bashrc:
#   source ~/march-madness/scripts/alias.sh

alias dmm_frontend='cd ~/march-madness && git pull && bun install && bun build:web'
alias dmm_backend='cd ~/march-madness && git pull && cargo build --release && sudo supervisorctl restart all'
alias dmm_all='cd ~/march-madness && git pull && bun install && bun build:web && cargo build --release && sudo supervisorctl restart all'
alias dmm_backfill='cd ~/march-madness && cargo build --release && ./target/release/march-madness-indexer backfill --from-block 30749805'
alias dmm_listen='cd ~/march-madness && cargo build --release && ./target/release/march-madness-indexer listen'
alias dmm_status='sudo supervisorctl status && curl -sf http://localhost:3000/health && echo " OK"'
