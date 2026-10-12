# epoll and eventfd are Linux-only, so everything runs in a Linux container.
# Works the same on macOS and Linux (Docker required).

.PHONY: help run client test stop

IMAGE     := rust:1.96
NAME      := async-epoll
MSG       ?= hello
# Named volumes cache downloaded crates and build output between runs.
DOCKER_RUN := docker run --rm --init --name $(NAME) -v "$(CURDIR)":/src -w /src \
	-v async-epoll-cargo:/usr/local/cargo/registry \
	-v async-epoll-target:/tmp/target -e CARGO_TARGET_DIR=/tmp/target $(IMAGE)
# The server listens on 127.0.0.1:10000 inside the container, so clients connect
# from inside it with bash's /dev/tcp.
CLIENT = exec 3<>/dev/tcp/127.0.0.1/10000; echo "$(1)" >&3; head -n1 <&3

help: ## Show available targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  %-8s %s\n", $$1, $$2}'

run: ## Start the echo server in the foreground (Ctrl-C to stop)
	$(DOCKER_RUN) cargo run

client: ## Send MSG (default: hello) to the running server and print the echo
	docker exec $(NAME) bash -c '$(call CLIENT,$(MSG))'

test: ## Start the server, run two concurrent clients, check idle CPU, then stop
	@$(DOCKER_RUN) cargo build -q
	@docker run -d --rm --init --name $(NAME) -v "$(CURDIR)":/src -w /src \
		-v async-epoll-target:/tmp/target $(IMAGE) /tmp/target/debug/async-epoll >/dev/null
	@sleep 1
	@docker exec $(NAME) bash -c '($(call CLIENT,hello from A)) & ($(call CLIENT,hello from B)) & wait'
	@sleep 2
	@echo "--- idle CPU of the server (should be about 0%):"
	@docker exec $(NAME) ps -o %cpu=,cmd= -C async-epoll
	@echo "--- server log:"
	@docker logs $(NAME)
	@docker stop $(NAME) >/dev/null

stop: ## Stop the server container
	docker stop $(NAME)
