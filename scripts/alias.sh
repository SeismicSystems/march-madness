# March Madness deploy aliases — source from ~/.bashrc:
#   source ~/march-madness/scripts/alias.sh

# Build only the 4 prod binaries (server, indexer, forecaster, ncaa-feed)
alias dmm_build='cargo build --release -p march-madness-server -p march-madness-indexer -p march-madness-forecaster -p ncaa-feed'

alias dmm_frontend='cd ~/march-madness && git pull && bun install && bun build:web'
alias dmm_backend='cd ~/march-madness && git pull && dmm_build && sudo supervisorctl restart all'
alias dmm_all='cd ~/march-madness && git pull && bun install && bun build:web && dmm_build && sudo supervisorctl restart all'
alias dmm_backfill='cd ~/march-madness && dmm_build && ./target/release/march-madness-indexer backfill --from-block 30749805'
alias dmm_listen='cd ~/march-madness && dmm_build && ./target/release/march-madness-indexer listen'
alias dmm_status='sudo supervisorctl status && curl -sf http://localhost:3000/health && echo " OK"'
