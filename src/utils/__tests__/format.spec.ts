import { describe, it, expect } from 'vitest'
import { formatFileSize } from '../format'

describe('formatFileSize', () => {
  it('null/undefined/0 返回 0 B', () => {
    expect(formatFileSize(null)).toBe('0 B')
    expect(formatFileSize(undefined)).toBe('0 B')
    expect(formatFileSize(0)).toBe('0 B')
  })

  it('字节级数值保留 B 单位', () => {
    expect(formatFileSize(512)).toBe('512 B')
    expect(formatFileSize(1023)).toBe('1023 B')
  })

  it('1024 字节为 1 KB', () => {
    expect(formatFileSize(1024)).toBe('1 KB')
    expect(formatFileSize(1536)).toBe('1.5 KB')
  })

  it('MB 级数值正确换算', () => {
    expect(formatFileSize(1048576)).toBe('1 MB')
    expect(formatFileSize(5 * 1024 * 1024)).toBe('5 MB')
  })

  it('GB 级数值正确换算', () => {
    expect(formatFileSize(1073741824)).toBe('1 GB')
    expect(formatFileSize(2 * 1024 * 1024 * 1024)).toBe('2 GB')
  })

  it('保留两位小数', () => {
    // 1.5 MB = 1572864 字节，应为 1.5 MB（已是最简）
    expect(formatFileSize(1572864)).toBe('1.5 MB')
    // 1.234567 MB 应四舍五入到 1.23 MB
    expect(formatFileSize(1293119)).toBe('1.23 MB')
  })
})
