import { onDestroy, onMount } from 'svelte'
import { writable, get } from 'svelte/store'
import { NETWORK_CHECK } from '../constants'

export interface NetworkStatus {
  online: boolean
  effectiveType?: string
  rtt?: number
  downlink?: number
  lastChecked?: Date
}

export const useNetwork = () => {
  const isOnline = writable<boolean>(navigator.onLine)
  const effectiveType = writable<string | undefined>(undefined)
  const rtt = writable<number | undefined>(undefined)
  const downlink = writable<number | undefined>(undefined)
  const lastChecked = writable<Date>(new Date())

  const updateNetworkStatus = () => {
    const connection = (navigator as any).connection
    if (connection) {
      effectiveType.set(connection.effectiveType)
      rtt.set(connection.rtt)
      downlink.set(connection.downlink)
    }
    isOnline.set(navigator.onLine)
    lastChecked.set(new Date())
  }

  const handleOnline = () => {
    isOnline.set(true)
    updateNetworkStatus()
  }

  const handleOffline = () => {
    isOnline.set(false)
    lastChecked.set(new Date())
  }

  const startMonitoring = () => {
    window.addEventListener('online', handleOnline)
    window.addEventListener('offline', handleOffline)

    const connection = (navigator as any).connection
    if (connection) {
      connection.addEventListener('change', updateNetworkStatus)
    }
  }

  const stopMonitoring = () => {
    window.removeEventListener('online', handleOnline)
    window.removeEventListener('offline', handleOffline)

    const connection = (navigator as any).connection
    if (connection) {
      connection.removeEventListener('change', updateNetworkStatus)
    }
  }

  const checkStatus = (): NetworkStatus => {
    updateNetworkStatus()
    return {
      online: get(isOnline),
      effectiveType: get(effectiveType),
      rtt: get(rtt),
      downlink: get(downlink),
      lastChecked: get(lastChecked)
    }
  }

  onMount(() => {
    updateNetworkStatus()
    startMonitoring()
  })

  onDestroy(() => {
    stopMonitoring()
  })

  return {
    isOnline,
    effectiveType,
    rtt,
    downlink,
    lastChecked,
    checkStatus,
    startMonitoring,
    stopMonitoring
  }
}

export default useNetwork
