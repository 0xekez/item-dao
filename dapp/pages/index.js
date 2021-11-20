import Head from 'next/head'
import { CosmWasmClient} from '@cosmjs/cosmwasm-stargate'
import {useState} from 'react'

let client = null

const getClient = async () => {
    if (!client) {
	client = await CosmWasmClient.connect('https://rpc.uni.junomint.com:443')
    }
    return client
}

const propGetter = async (setProps) => {
    const client = await getClient()
    const props = await client.queryContractSmart('juno1eu70kcgh0d2rlm0n88dgtry9wpqnerf5n2fdzt5sxm6d3vrqq3xqa5e9x8', "list_proposals")
    setProps(props)
}

const itemGetter = async (setItems) => {
    const client = await getClient()
    const items = await client.queryContractSmart('juno1eu70kcgh0d2rlm0n88dgtry9wpqnerf5n2fdzt5sxm6d3vrqq3xqa5e9x8', "list_items")
    setItems(items)
}

const getColor = (idx) => {
    const colors = ["#9400D3", "indigo", "#0047AB", "#378642", "#e3b104"]
    return colors[idx % colors.length]
}

const statusToSymbol = (status) => {
    switch (status) {
    case "Passed":
	return "👍"
    case "Failed":
	return "👎"
    case "Pending":
	return "🤷"
    default:
	return "🦄"
    }
}

export default function Home() {
    const [props, setProps] = useState([])
    propGetter(setProps)
    const [items, setItems] = useState([])
    itemGetter(setItems)

    return (
	    <html>
	    <Head>
            <title>item-dao!</title>
            <link rel="icon" href="https://moonphase.is/image.svg" />
	    </Head>

	    <main>
            <h1>
            Welcome to <a href="https://github.com/ezekiiel/item-dao" target="_blank">item-dao!</a>
            </h1>

	    <h2>Items</h2>
	    {items.map((item, idx) => {
		const style = { color: getColor(idx) }
		return (
			<div key={`{item.name}_{idx}`} className="item" style={style}>
			<h3>{item.name}</h3>
			<p>{item.contents}</p>
			</div>
		)
	    })}

	    <h2>Proposals</h2>
	    {props.map((prop, idx) => {
		const style = { color: getColor(idx + items.length) }
		return (
			<div key={`{prop.title}_{idx}`} className="item" style={style}>
			<h3>{prop.title}</h3>
			<h4>status {statusToSymbol(prop.status)}</h4>
			<p>{prop.body}</p>
			</div>
		)
	    })}

	    <style>{`
html {
    font-family: sans-serif;
    max-width: 600px;
    padding: 6px;
    line-height: 1.5;
    margin: auto;
}

h1 {
    font-size: 225%;
}

p, ul, ol {
    font-size: 18px;
}

a {
    color: #0000bb;
}

img, video {
    max-width: 100%;
}

.item {
   margin: 16px 0px;
   padding: 2px;
   margin-left: 16px;
}

.item h3 {
    margin: 4px 0px;
}
.item h4 {
    margin: 0;
}
.item p {
    margin: 8px 0px;
}
      `}</style>
	    </main>
	    </html>
    )
}
