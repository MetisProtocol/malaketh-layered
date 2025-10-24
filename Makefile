all: clean
	cargo build
	#cargo run --bin malachitebft-eth-utils genesis --validator-config nodes_config_bin/0/config/genesis.json
	docker compose up -d
#	./scripts/add_peers.sh 
#	cp -fr nodes_config nodes	
	cp -fr nodes_config_bin nodes	
#	docker compose -f compose-mala.yaml up -d
#	cargo run --bin malachitebft-eth-app -- testnet --nodes 3 --home nodes
#	echo 👉 Grafana dashboard is available at http://localhost:3000
	bash scripts/spawn.bash --nodes 3 --home nodes

stop:
	docker compose down

clean: clean-prometheus
	rm -rf ./nodes
	rm -rf ./rethdata
	rm -rf ./monitoring/data-grafana

clean-prometheus: stop
	rm -rf ./monitoring/data-prometheus

spam:
	cargo run --bin malachitebft-eth-utils spam --time=60 --rate=500 --rpc-url=127.0.0.1:8545

add-new-peer:
	bash scripts/spawn-new-peer.bash
