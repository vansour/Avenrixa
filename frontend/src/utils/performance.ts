interface CacheEntry<T> {
    value: T
    timestamp: number
}

class LRUCache<K, V> {
    private cache: Map<K, CacheEntry<V>> = new Map()
    private maxEntries: number

    constructor(maxEntries: number = 100) {
        this.maxEntries = maxEntries
    }

    get(key: K): V | undefined {
        const entry = this.cache.get(key)
        if (entry) {
            entry.timestamp = Date.now()
            return entry.value
        }
        return undefined
    }

    set(key: K, value: V, ttl: number): void {
        const entry: CacheEntry<V> = {
            value,
            timestamp: Date.now()
        }

        this.cache.set(key, entry)

        if (this.cache.size > this.maxEntries) {
            this.evictOldest()
        }

        this.evictExpired(ttl)
    }

    private evictOldest(): void {
        let oldestKey: K | undefined = undefined
        let oldestTime = Date.now()

        for (const [key, entry] of this.cache) {
            if (entry.timestamp < oldestTime) {
                oldestTime = entry.timestamp
                oldestKey = key
            }
        }

        if (oldestKey !== undefined) {
            this.cache.delete(oldestKey)
        }
    }

    private evictExpired(ttl: number): void {
        const now = Date.now()
        for (const [key, entry] of this.cache) {
            if (now - entry.timestamp > ttl) {
                this.cache.delete(key)
            }
        }
    }

    has(key: K): boolean {
        return this.cache.has(key)
    }

    clear(): void {
        this.cache.clear()
    }

    size(): number {
        return this.cache.size
    }
}

const imageCache = new LRUCache<string, any>(100)
const textCache = new LRUCache<string, any>(50)
const dataCache = new LRUCache<string, any>(30)

export function memoize<T extends (...args: any[]) => any>(
    key: string,
    fn: T,
    ttl: number = 300000,
    cache: LRUCache<any, any> = imageCache
): T {
    function wrapped(...args: any[]): any {
        const cached = cache.get(key)
        if (cached && Date.now() - cached.timestamp < ttl) {
            return cached.value
        }

        const result = fn(...args)
        cache.set(key, result, ttl)
        return result
    }
    return wrapped as T
}

export function clearCache(cache: LRUCache<any, any>, key?: string): void {
    if (key) {
        cache.forEach((value, cacheKey) => {
            if (cacheKey === key) {
                cache.delete(cacheKey)
            }
        })
    } else {
        cache.clear()
    }
}

export async function batchProcess<T, R>(
    items: T[],
    processFn: (item: T, index: number) => Promise<R> | R,
    batchSize: number = 10
): Promise<R[]> {
    const results: R[] = []
    for (let i = 0; i < items.length; i += batchSize) {
        const batch = items.slice(i, i + batchSize)
        await new Promise(resolve => setTimeout(resolve, 0))
        const batchResults = await Promise.all(
            batch.map((item, index) => Promise.resolve(processFn(item, i + index)))
        )
        results.push(...batchResults)
    }
    return results
}

export function runWhenIdle(callback: () => void, timeout?: number): void {
    if ('requestIdleCallback' in window) {
        (window as any).requestIdleCallback(() => callback(), { timeout })
    } else {
        setTimeout(callback, timeout || 100)
    }
}

export function asyncDebounce<T extends (...args: any[]) => Promise<any>>(
    fn: T,
    delay: number = 300
): (...args: any[]) => Promise<ReturnType<T>> {
    let timeoutId: ReturnType<typeof setTimeout> | null = null
    let pending: Promise<ReturnType<T>> | null = null

    const debounced = function(...args: any[]): Promise<ReturnType<T>> {
        if (timeoutId) {
            clearTimeout(timeoutId)
        }

        const promise = new Promise<ReturnType<T>>((resolve, reject) => {
            timeoutId = setTimeout(() => {
                fn(...args).then(resolve).catch(reject)
            }, delay)
        })

        if (pending) {
            pending.then(resolve).catch(reject)
        }

        pending = promise
        return promise
    }

    return debounced
}

export function preloadImages(urls: string[]): void {
    urls.forEach((url) => {
        const img = new Image()
        img.onload = () => {
            URL.revokeObjectURL(url)
        }
        img.src = url
    })
}

export interface VirtualScrollItem {
    id: string | number
    height: number
}

export class VirtualScroll<T extends VirtualScrollItem> {
    private containerHeight: number = 0
    private scrollTop: number = 0
    private startIndex: number = 0
    private endIndex: number = 0
    private buffer: number = 5
    private visibleRange: [number, number] | null = null

    constructor(
        private items: T[],
        private itemHeight: number,
        private renderCallback: (visibleItems: T[]) => void
    ) {}

    updateItems(items: T[]) {
        this.items = items
        this.visibleRange = null
        this.updateVisibleRange()
    }

    setContainerHeight(height: number) {
        this.containerHeight = height
        this.updateVisibleRange()
    }

    setScrollTop(scrollTop: number) {
        this.scrollTop = scrollTop
        this.updateVisibleRange()
    }

    setBuffer(buffer: number) {
        this.buffer = buffer
        this.updateVisibleRange()
    }

    private updateVisibleRange(): void {
        if (this.visibleRange) {
            const [oldStart, oldEnd] = this.visibleRange
            const newStart = Math.max(0, Math.floor(this.scrollTop / this.itemHeight) - this.buffer)
            const newEnd = Math.min(
                this.items.length - 1,
                newStart + Math.ceil(this.containerHeight / this.itemHeight) + this.buffer * 2
            )

            if (oldStart !== newStart || oldEnd !== newEnd) {
                this.visibleRange = [newStart, newEnd]
                this.renderCallback(this.items.slice(newStart, newEnd + 1))
            }
        } else {
            this.updateVisibleRangeImpl()
        }
    }

    private updateVisibleRangeImpl(): void {
        const startIndex = Math.max(0, Math.floor(this.scrollTop / this.itemHeight) - this.buffer)
        const endIndex = Math.min(
            this.items.length - 1,
            startIndex + Math.ceil(this.containerHeight / this.itemHeight) + this.buffer * 2
        )
        this.visibleRange = [startIndex, endIndex]
        this.renderCallback(this.items.slice(startIndex, endIndex + 1))
    }

    getTotalHeight(): number {
        return this.items.length * this.itemHeight
    }
}

export class WorkerQueue {
    private queue: Array<() => Promise<any>> = []
    private activeWorkers = 0
    private maxWorkers: number
    private maxQueueSize: number = 50

    constructor(maxWorkers: number = 4) {
        this.maxWorkers = maxWorkers
    }

    add<T>(task: () => Promise<T>): Promise<T> {
        if (this.queue.length >= this.maxQueueSize) {
            this.queue.shift()
        }

        return new Promise<T>((resolve, reject) => {
            this.queue.push(async () => {
                await task().then(resolve).catch(reject)
            })
            this.process()
        })
    }

    private async process(): Promise<void> {
        if (this.activeWorkers >= this.maxWorkers || this.queue.length === 0) {
            return
        }

        const task = this.queue.shift()
        if (!task) return

        this.activeWorkers++
        try {
            await (task as any)()
        } finally {
            this.activeWorkers--
            this.process()
        }
    }

    size(): number {
        return Math.min(this.queue.length, this.maxQueueSize)
    }
}

export function isLowEndDevice(): boolean {
    const ua = navigator.userAgent.toLowerCase()
    return (
        /android|webos|iphone|ipad|ipod|blackberry|iemobile|opera mini/i.test(ua) ||
        (navigator as any).hardwareConcurrency <= 2
    )
}

export function getPerformanceConfig() {
    const isLowEnd = isLowEndDevice()
    return {
        lazyLoadThreshold: isLowEnd ? 200 : 50,
        virtualScrollBuffer: isLowEnd ? 3 : 5,
        batchSize: isLowEnd ? 5 : 10,
        cacheTTL: isLowEnd ? 600000 : 300000,
        animationEnabled: !isLowEnd,
        prefetchEnabled: !isLowEnd
    }
}
