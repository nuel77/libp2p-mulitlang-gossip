import logo from './logo.svg';
import './App.css';
import {createLibp2p} from "libp2p";
import {webSockets} from "@libp2p/websockets"
import { webRTCStar } from '@libp2p/webrtc-star'
import {useEffect} from "react";
import { bootstrap } from '@libp2p/bootstrap'
import {noise} from "@chainsafe/libp2p-noise"
import {mplex} from "@libp2p/mplex"
import { multiaddr } from 'multiaddr'
import {gossipsub, GossipSub} from "@chainsafe/libp2p-gossipsub"
import { kadDHT } from '@libp2p/kad-dht'
import { floodsub } from '@libp2p/floodsub'

import { fromString as uint8ArrayFromString } from 'uint8arrays/from-string'
import { toString as uint8ArrayToString } from 'uint8arrays/to-string'

const bootstraps = [
  "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
  "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
  "/dnsaddr/bootstrap.libp2p.io/p2p/QmZa1sAxajnQjVM8WjWXoMbmPd7NsWhfKsPkErzpm9wGkp",
  "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
  "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
]

function App() {
  const createNode = async ()=>{
    const wrtcStar = webRTCStar()
    const node = await createLibp2p({
      addresses: {
        listen: [
            "/dns4/wrtc-star1.par.dwebops.pub/tcp/443/wss/p2p-webrtc-star",
          "/dns4/wrtc-star2.sjc.dwebops.pub/tcp/443/wss/p2p-webrtc-star",
        ]
      },
      transports: [
        webSockets(),
        wrtcStar.transport
      ],
      connectionEncryption: [
        noise()
      ],
      streamMuxers: [mplex()],
      peerDiscovery: [
        bootstrap({
          interval: 100,
          list: bootstraps
        })
      ],
      pubsub: gossipsub( {allowPublishToZeroPeers: true, emitSelf: true  }),
      dht: kadDHT(),
    })
    node.connectionManager.addEventListener('peer:connect', (evt) => {
      const connection = evt.detail
      console.log('Connection established to:', connection.remotePeer.toString())	// Emitted when a peer has been found
    })
    node.addEventListener('peer:discovery', (evt) => {
      const peer = evt.detail
      // No need to dial, autoDial is on
      // console.log('Discovered:', peer.id.toString())
    })
    await node.start()
    console.log("node started : ", node.peerId.toCID().toString())
    // print out listening addresses
    const listenAddrs = node.getMultiaddrs()
    console.log('listening on addresses:', listenAddrs.map(i=>i.toString()))


    let remoteMultiaddr=""
    remoteMultiaddr="/dns4/wrtc-star1.par.dwebops.pub/tcp/443/wss/p2p-webrtc-star/p2p/QmYkMYKA9NfFfLuf9RiRaEoKKquYKtz2zdGsyZupigMTKf"
    if(remoteMultiaddr){
      const ma = multiaddr(remoteMultiaddr)
      console.log(`pinging remote peer at ${remoteMultiaddr}`)
      const latency = await node.ping(ma)
      console.log(`pinged ${remoteMultiaddr} in ${latency}ms`)
    }
    const topic = 'chat-gossip'
    await node.pubsub.subscribe(topic)
    node.pubsub.addEventListener("message", (evt) => {
      console.log(`node1 received: ${uint8ArrayToString(evt.detail.data)} on topic ${evt.detail.topic}`)
    })
// Publish a new message each second
    setInterval(async () => {
      await node.pubsub.publish(topic, uint8ArrayFromString('Bird bird bird, bird is the word!'))
    }, 5000)

  }
  useEffect(()=>{
    createNode()
  },[])

  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/App.js</code> and save to reload.
        </p>
        <a
          className="App-link"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
      </header>
    </div>
  );
}

export default App;
