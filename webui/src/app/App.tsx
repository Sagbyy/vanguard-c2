import { TacticalPage } from '@/pages/tactical'
import { useTelemetry } from './model'

export default function App() {
  const telemetry = useTelemetry()
  return <TacticalPage {...telemetry} />
}
