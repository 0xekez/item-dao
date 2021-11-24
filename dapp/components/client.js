import { CosmWasmClient} from '@cosmjs/cosmwasm-stargate'

let client = null

export const getClient = async () => {
    if (!client) {
	client = await CosmWasmClient.connect('https://rpc.uni.junomint.com:443')
    }
    return client
}
