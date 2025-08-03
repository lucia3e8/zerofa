web app that reads my emails and if it sees a 2fa code email, makes it visible on the site

website is a rust app using a minimal http stack, no async especially no tokio.
to determine if an email is a 2fa email, use a regex that matches this title: "Your <servicename> code is <code>"

the app should be notified of new emails fairly quickly. should we use pop3 or imap? explain the alternatives
