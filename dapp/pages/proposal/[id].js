import {getClient} from '../../components/client.js'
import {useState} from 'react'
import Layout from '../../components/layout.js'

import { useRouter } from 'next/router'
import Head from 'next/head'
import Link from 'next/link'
import { Flex, NavLink, Box } from 'theme-ui'
import { Spinner, Heading, Paragraph, Message } from 'theme-ui'

let loading = true

const getProp = async (id, setProp) => {
    const client = await getClient()
    const prop = await client.queryContractSmart('juno1eu70kcgh0d2rlm0n88dgtry9wpqnerf5n2fdzt5sxm6d3vrqq3xqa5e9x8', {"get_proposal": { "proposal_id": parseInt(id)}})
    loading = false
    setProp(prop)
}

export default function Proposal() {
    const router = useRouter()
    const { id } = router.query

    const [prop, setProp] = useState({})

    if (id) {
	getProp(id, setProp)
    }

    if (loading) {
	return (
	    <Layout>
		<Heading as='h3'>proposal {id}</Heading>
		<Spinner/>
	    </Layout>
	)
    }

    return (
	<Layout>
	    <Flex>
		<Heading as='h3' sx={{mt: 2, mr: 1}}><Link href='/'>item-dao</Link></Heading>
		<Heading as='h3' sx={{mt: 2, mr: 1}}>|</Heading>
		<Heading as='h3' sx={{mt: 2, mr: 1}}>proposal {id}</Heading>
	    </Flex>
	    <Heading variant="text.display">
		{prop ? prop.title : "Proposal {id}"}
	    </Heading>
	    <Paragraph>
		{prop.body}
	    </Paragraph>
	    <Heading variant="text.heading" sx={{mt: 2}}>action</Heading>
	    <Box p={4} color="white" bg="muted" sx={{fontFamily: "monospace", mt: 2}}>
		{prop.action ? JSON.stringify(prop.action) : ""}
	    </Box>
	</Layout>
    )
}