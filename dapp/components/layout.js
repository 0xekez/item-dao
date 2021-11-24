import Head from 'next/head'
import Link from 'next/link'
import { Flex, NavLink } from 'theme-ui'
import { ThemeProvider } from 'theme-ui'
import { theme } from './theme.js'

export default function Layout({ children }) {
    return(
	<ThemeProvider theme={theme}>
	    <Head>
		<title>item-dao!</title>
		<link rel="icon" href="https://moonphase.is/image.svg" />
	    </Head>
	    <main>
		{children}
	    </main>
	</ThemeProvider>
    )
}
