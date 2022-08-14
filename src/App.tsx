import { invoke } from '@tauri-apps/api/tauri'
import { useCallback, useEffect, useState } from 'react'
import './App.css';
import { Buffer } from "buffer";

type MetaData = {
    path: string,
    title: string | null,
    artist: string | null,
    group: string | null,
    album: string | null,
    trackNumber: string | null,
    visual: {
        mediaType: string,
        data: number[]
    } | null
}

function App() {
    const [tracks, setTracks] = useState<MetaData[]>();
    const getPaths = useCallback(async () => {
        const tracks = await invoke<MetaData[]>('find_files');
        setTracks(tracks.sort((a, b) => a.trackNumber?.localeCompare(b.trackNumber ?? '0') ?? 0));
    }, [])

    useEffect(() => {
        getPaths();
    }, []);

    return (
        <div>
            <div>
                <button onClick={() => {
                    invoke('pause_play');
                }}>pause</button>
            </div>
            <div>
                {tracks?.map((track) =>
                    <div
                        key={`${track.artist}${track.title}`}
                        onClick={() => {
                            invoke('play_file', { path: track.path });
                        }}
                    >
                        <div>{track.title}</div>
                        <div>{track.artist}</div>
                        <div>{track.album}</div>
                        <div>{track.trackNumber}</div>
                        <img
                            src={`data:${track.visual?.mediaType};base64,${Buffer.from(track.visual?.data ?? []).toString('base64')}`}
                            width="320"
                            height="320"
                        />
                    </div>
                )}
            </div>
        </div>
    )
}

export default App
