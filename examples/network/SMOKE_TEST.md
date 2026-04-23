# Network Smoke Test

## Terminal 1
./examples/network/registry_a.sh

## Terminal 2
./examples/network/registry_b.sh

## Terminal 3
./examples/network/node_a.sh

## Terminal 4
./examples/network/node_b.sh

## Terminal 5
./examples/network/http_a.sh

## Terminal 6
./examples/network/http_b.sh

## Terminal 7
curl http://127.0.0.1:8081/v1/info

## Terminal 8
curl http://127.0.0.1:8082/v1/info
