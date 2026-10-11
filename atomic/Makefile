.PHONY: help run reorder relaxed acqrel

help: ## Show available targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  %-10s %s\n", $$1, $$2}'

run: ## Count how often the reader sees READY as true
	cargo run --release

reorder: relaxed acqrel ## Run the reorder test in both modes

relaxed: ## Reorder test with Relaxed on READY
	cargo run -q --release --example reorder -- relaxed

acqrel: ## Reorder test with Release/Acquire on READY
	cargo run -q --release --example reorder -- acqrel
