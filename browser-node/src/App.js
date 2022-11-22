import logo from './logo.svg';
import './App.css';
import {createLibp2p} from "libp2p";
import {webSockets} from "@libp2p/websockets"
import {useEffect} from "react";
import {noise} from "@chainsafe/libp2p-noise"
import {mplex} from "@libp2p/mplex"
function App() {
  const createNode= async ()=>{
    const node = await createLibp2p({
      transports: [
        webSockets()
      ],
      connectionEncryption: [
        noise()
      ],
      streamMuxers: [mplex()]
    })
    await node.start()
    const connections =  node.getConnections()
    console.log("connections", connections)
    console.log("isStarted", node.isStarted())
  }
  useEffect(()=>{createNode()},[])
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
