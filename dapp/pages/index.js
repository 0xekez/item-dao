import {useState} from 'react'
import {getClient} from '../components/client.js'
import Item from '../components/item.js'
import Prop from '../components/prop.js'
import Layout from '../components/layout.js'
import { Spinner, Heading } from 'theme-ui'
import Link from 'next/link'

let loadingProps = true
let loadingItems = true

const propGetter = async (setProps) => {
    const client = await getClient()
    const props = await client.queryContractSmart('juno1skuakpnx8gec0avqdr00nl5339k5nc6rplsnr4gtv3ge3q8e2l2qwayda3', "list_proposals")
    loadingProps = false
    setProps(props)
}

const itemGetter = async (setItems) => {
    const client = await getClient()
    const items = await client.queryContractSmart('juno1skuakpnx8gec0avqdr00nl5339k5nc6rplsnr4gtv3ge3q8e2l2qwayda3', "list_items")
    loadingItems = false
    setItems(items)
}

export default function Home() {
    const [props, setProps] = useState([])
    propGetter(setProps)
    const [items, setItems] = useState([])
    itemGetter(setItems)

    return (
	<Layout>
            <Heading variant="text.display">
		Welcome to <a href="https://github.com/ezekiiel/item-dao" target="_blank">item-dao!</a>
            </Heading>
	    <Heading as="h2">Items</Heading>
	    {loadingItems ? <Spinner/> : items.map((item, idx) => {
		return (
		    <Item
			name={item.name}
			contents={item.contents}
		    />
		)
	    })}

	    <Heading as="h2">Proposals</Heading>
	    {loadingProps ? <Spinner/> : props.map((prop, idx) => {
		return (
		    <Prop
			title={prop.title}
			status={prop.status}
			body={prop.body}
			id={idx}
		    />
		)
	    })}
	</Layout>
    )
}
