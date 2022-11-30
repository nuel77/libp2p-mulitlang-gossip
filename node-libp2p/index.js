import { createLibp2p } from 'libp2p'
import {gossipsub, GossipSub} from '@chainsafe/libp2p-gossipsub'
import { tcp } from '@libp2p/tcp'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'
import { fromString as uint8ArrayFromString } from "uint8arrays/from-string";
import { toString as uint8ArrayToString } from "uint8arrays/to-string";
import { kadDHT } from '@libp2p/kad-dht'
import {multiaddr} from "multiaddr";
import { peerIdFromString } from '@libp2p/peer-id'
import {webSockets} from "@libp2p/websockets"
import { yamux } from '@chainsafe/libp2p-yamux'
import { floodsub } from '@libp2p/floodsub'

const main = async () =>{
    const topic = 'gossip';

    const node1 =await createNode();
    console.log("node1 peer id", node1.getMultiaddrs().map(i=>i.toString()))
    // Add node's 2 data to the PeerStore
    const node2 =  getOuterNode();
    await node1.peerStore.addressBook.set(node2.peerId, node2.multiaddr);

    await node1.dial(node2.peerId)

    node1.pubsub.addEventListener("message", (evt) => {
        console.log(`node1 received: ${uint8ArrayToString(evt.detail.data)} on topic ${evt.detail.topic}`)
    })
    await node1.pubsub.subscribe(topic)

    process.stdin.on("data", function(data) {
        console.log("sending : ", data.toString());
        node1.pubsub.publish(topic, uint8ArrayFromString(data.toString())).then(()=>console.log("sent")).catch(err => {
            console.error(err)
        })
    });
}

const createNode = async () => {
    const node = await createLibp2p({
        addresses: {
            listen: ['/ip4/0.0.0.0/tcp/0']
        },
        transports: [tcp(), webSockets()],
        streamMuxers: [yamux()],
        connectionEncryption: [noise()],
        // we add the Pubsub module we want
        pubsub:gossipsub({ allowPublishToZeroPeers: true }),//floodsub(),
        dht: kadDHT()
    })
    node.connectionManager.addEventListener('peer:connect', (evt) => {
        const connection = evt.detail
        console.log('Connection established to:', connection.remotePeer.toString())	// Emitted when a peer has been found
    });
    await node.start()
    return node
}
const getOuterNode= ()=>{
    const args= process.argv.slice(2);
    const peerIdStr= args[0];
    const multiaddrStr = args[1];
    return {
        peerId:peerIdFromString(peerIdStr),
        multiaddr: [multiaddr(multiaddrStr)]
    }
}

main().then(()=>console.log("started..."))