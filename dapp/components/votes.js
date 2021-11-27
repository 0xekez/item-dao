import { Card, Heading, Flex, Text } from 'theme-ui'

function Vote(props) {
    return (
	<Card mt={2} mb={2} key={props.addr + props.pos + props.amount} p={2} bg={props.pos}>
	    <Heading>{props.pos}</Heading>
	    <Heading as='p'>tokens: {props.amount}</Heading>
	    <Text>address: {props.addr}</Text>
	</Card>
    )
}

export default function Votes(props) {
    if (!props.yes || !props.no || !props.abstain) {
	return null
    }
    return (
	<>
	    {props.yes.map((vote) => {
		return (<Vote amount={vote[1]} addr={vote[0]} pos='yes'/>)
	    })}
	    {props.no.map((vote) => {
		return (<Vote amount={vote[1]} addr={vote[0]} pos='no'/>)
	    })}
	    {props.abstain.map((vote) => {
		return (<Vote amount={vote[1]} addr={vote[0]} pos='abstain'/>)
	    })}
	</>
    )
}
