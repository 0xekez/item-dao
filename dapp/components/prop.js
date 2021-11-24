import { Box, Heading, Paragraph, Button } from 'theme-ui'
import Link from 'next/link'

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

export default function Prop(props) {
    const href = '/proposal/' + props.id
    return (
	<Box m={1} p={3} color="white" bg="muted" sx={{overflow: "scroll"}}>
	    <Heading as="h3"><Link href={href}>{props.title}</Link></Heading>
	    <Heading as="h4">{statusToSymbol(props.status)}</Heading>
	    <Paragraph>{props.body}</Paragraph>
	</Box>
    )
}
