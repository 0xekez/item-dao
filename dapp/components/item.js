import { Box, Heading, Paragraph } from 'theme-ui'

export default function Item(props) {
    return (
	<Box m={1} p={3} color="white" bg="muted">
	    <Heading as="h3">{props.name}</Heading>
	    <Paragraph>{props.contents}</Paragraph>
	</Box>
    )
}
