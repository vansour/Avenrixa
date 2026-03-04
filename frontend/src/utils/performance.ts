interface CacheEntry<T> {
    value: T
    timestamp: number
}

/**
 * 缓存统计信息
 */
export interface CacheStats {
    hits: number
    misses: number
    size: number
    get hitRate(): number
}

/**
 * 缓存配置选项
 */
export interface CacheOptions {
    maxEntries?: number
    persistKey?: string
    persistTTL?: number
    enableStats?: boolean
}

/**
 * 缓存持久化数据
 */
interface PersistedCacheData {
    entries: Array<{ key: string; value: any; timestamp: number }>
    timestamp: number
}

class LRUCache<K, V> {
    private cache: Map<K, CacheEntry<V>> = new Map()
    private maxEntries: number
    private stats: { hits: number; misses: number } = { hits: 0, misses: 0 }
    private persistKey: string | null = null
    private persistTTL: number = 3600000 // 默认1小时
    private enableStats: boolean = false

    constructor(maxEntries: number = 100, options?: CacheOptions) {
        this.maxEntries = maxEntries

        if (options) {
            this.persistKey = options.persistKey || null
            this.persistTTL = options.persistTTL || 3600000
            this.enableStats = options.enableStats || false

            // 如果启用了持久化，尝试从 localStorage 加载
            if (this.persistKey) {
                this.loadFromStorage()
            }
        }
    }

    get(key: K): V | undefined {
        const entry = this.cache.get(key)
        if (entry) {
            entry.timestamp = Date.now()
            if (this.enableStats) {
                this.stats.hits++
            }
            return entry.value
        }
        if (this.enableStats) {
            this.stats.misses++
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

        // 如果启用了持久化，保存到 localStorage
        if (this.persistKey) {
            this.saveToStorage()
        }
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

    delete(key: K): boolean {
        const deleted = this.cache.delete(key)
        if (deleted && this.persistKey) {
            this.saveToStorage()
        }
        return deleted
    }

    clear(): void {
        this.cache.clear()
        if (this.enableStats) {
            this.stats = { hits: 0, misses: 0 }
        }
        if (this.persistKey) {
            this.clearStorage()
        }
    }

    size(): number {
        return this.cache.size
    }

    /**
     * 获取缓存统计信息
     */
    getStats(): CacheStats {
        return {
            hits: this.stats.hits,
            misses: this.stats.misses,
            size: this.cache.size,
            get hitRate() {
                const total = this.hits + this.misses
                return total === 0 ? 0 : (this.hits / total) * 100
            }
        }
    }

    /**
     * 重置统计信息
     */
    resetStats(): void {
        this.stats = { hits: 0, misses: 0 }
    }

    /**
     * 保存缓存到 localStorage
     */
    private saveToStorage(): void {
        if (!this.persistKey) return

        try {
            const data: PersistedCacheData = {
                entries: Array.from(this.cache.entries()).map(([key, entry]) => ({
                    key: String(key),
                    value: entry.value,
                    timestamp: entry.timestamp
                })),
                timestamp: Date.now()
            }
            localStorage.setItem(this.persistKey, JSON.stringify(data))
        } catch (e) {
            console.warn('Failed to save cache to localStorage:', e)
        }
    }

    /**
     * 从 localStorage 加载缓存
     */
    private loadFromStorage(): void {
        if (!this.persistKey) return

        try {
            const stored = localStorage.getItem(this.persistKey)
            if (!stored) return

            const data: PersistedCacheData = JSON.parse(stored)

            // 检查是否过期
            const age = Date.now() - data.timestamp
            if (age > this.persistTTL) {
                this.clearStorage()
                return
            }

            // 恢复缓存条目
            this.cache.clear()
            for (const entry of data.entries) {
                this.cache.set(entry.key as K, {
                    value: entry.value,
                    timestamp: entry.timestamp
                })
            }
        } catch (e) {
            console.warn('Failed to load cache from localStorage:', e)
            this.clearStorage()
        }
    }

    /**
     * 清除 localStorage 中的缓存
     */
    private clearStorage(): void {
        if (!this.persistKey) return

        try {
            localStorage.removeItem(this.persistKey)
        } catch (e) {
            console.warn('Failed to clear cache from localStorage:', e)
        }
    }

    /**
     * 预热缓存
     */
    async prewarm(entries: Array<{ key: K; value: V }>): Promise<void> {
        for (const entry of entries) {
            this.cache.set(entry.key, {
                value: entry.value,
                timestamp: Date.now()
            })
        }
        if (this.persistKey) {
            this.saveToStorage()
        }
    }

    /**
     * 遍历缓存条目
     */
    forEach(callback: (value: CacheEntry<V>, key: K) => void): void {
        this.cache.forEach(callback)
    }
}

/**
 * 图片缓存（带持久化和统计）
 */
const imageCache = new LRUCache<string, any>(100, {
    persistKey: 'img_cache_images',
    persistTTL: 7200000, // 2小时
    enableStats: true
})

/**
 * 文本缓存（带持久化和统计）
 */
const textCache = new LRUCache<string, any>(50, {
    persistKey: 'img_cache_text',
    persistTTL: 3600000, // 1小时
    enableStats: true
})

/**
 * 数据缓存（带持久化和统计）
 */
const dataCache = new LRUCache<string, any>(30, {
    persistKey: 'img_cache_data',
    persistTTL: 1800000, // 30分钟
    enableStats: true
})

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

/**
 * 批量处理（改进版：使用动态批次大小）
 */
export async function batchProcess<T, R>(
    items: T[],
    processFn: (item: T, index: number) => Promise<R> | R,
    batchSize: number = 10
): Promise<R[]> {
    const results: R[] = []
    const isLowEnd = isLowEndDevice()
    const dynamicBatchSize = isLowEnd ? Math.max(3, Math.floor(batchSize / 2)) : batchSize

    for (let i = 0; i < items.length; i += dynamicBatchSize) {
        const batch = items.slice(i, i + dynamicBatchSize)

        // 让出主线程
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

        const currentPromise = new Promise<ReturnType<T>>((resolve, reject) => {
            timeoutId = setTimeout(() => {
                fn(...args).then(resolve).catch(reject)
            }, delay)
        })

        if (pending) {
            pending.then(resolve).catch(reject)
        }

        pending = currentPromise
        return currentPromise
    }

    return debounced
}

export function preloadImages(urls: string[]): void {
    runWhenIdle(() => {
        urls.forEach((url, index) => {
            runWhenIdle(() => {
                const img = new Image()
                img.onload = () => {
                    URL.revokeObjectURL(url)
                }
                img.onerror = () => {
                    // 静默失败
                }
                img.src = url
            })
        })
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

    clear(): void {
        this.queue = []
    }

    stats() {
        return {
            queueSize: this.queue.length,
            activeWorkers: this.activeWorkers,
            maxWorkers: this.maxWorkers
        }
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

/**
 * 获取缓存统计信息
 */
export function getCacheStats(): {
    images: CacheStats
    text: CacheStats
    data: CacheStats
} {
    return {
        images: imageCache.getStats(),
        text: textCache.getStats(),
        data: dataCache.getStats()
    }
}

/**
 * 重置所有缓存统计
 */
export function resetAllCacheStats(): void {
    imageCache.resetStats()
    textCache.resetStats()
    dataCache.resetStats()
}

/**
 * 清除所有持久化缓存
 */
export function clearAllPersistentCache(): void {
    imageCache.clear()
    textCache.clear()
    dataCache.clear()
}

/**
 * 导出缓存实例
 */
export { imageCache, textCache, dataCache }
