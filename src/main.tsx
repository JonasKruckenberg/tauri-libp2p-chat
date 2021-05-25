import { render } from 'preact'
import { tauri, event } from '@tauri-apps/api'
import { useEffect, useState } from 'preact/hooks'

import 'virtual:windi.css'
import 'virtual:windi-devtools'

interface Message {
    message: string,
    from: string | null
}

const App = () => {
    const [messages, setMessages] = useState<Message[]>([])
    const [message, setMessage] = useState('')


    useEffect(() => {
        event.listen('plugin:libp2p|message', (msg: any) => {
            console.log(msg);
            
            setMessages(prev => [...prev, msg.payload])
        })
    }, [])

    function onSubmit(e: any) {
        setMessages(prev => [...prev, { message, from: null }])
        tauri.invoke('plugin:libp2p|broadcast', { message })
        setMessage('')

        e.preventDefault();
    }

    return <div class="flex flex-col container mx-auto h-screen overflow-hidden">
        <main class="flex flex-col flex-grow overflow-y-auto p-2">
            {messages.map(msg => {
                if (msg.from) {
                    return <span class="m-1">
                        <span class="text-xs text-gray-500 max-w-md truncate">
                            From: {msg.from.slice(msg.from.length - 8)}
                        </span>
                        <br />
                        <span class="p-1 rounded bg-teal-400 text-white min-w-md">
                            {msg.message}
                        </span>
                    </span>
                } else {
                    return <span class="m-1 self-end">
                        <span class="p-1 rounded bg-pink-400 text-white">
                            {msg.message}
                        </span>
                    </span>
                }
            })}
        </main>
        <form onSubmit={onSubmit} class="p-2">
            <input type="text" value={message} onInput={e => setMessage(e.currentTarget.value)} class="rounded bg-gray-100 p-2 m-1 border-2 border-gray-300" />
            <button type="submit" class="p-2 rounded bg-pink-400 text-white shadow-md">Send</button>
        </form>
    </div>
}
render(<App />, document.getElementById('app')!)
