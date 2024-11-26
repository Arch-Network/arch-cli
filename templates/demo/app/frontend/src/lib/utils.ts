
import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}


export function truncateBtcAddress(address: string, length: number = 10): string {
  return `${address.slice(0, length)}...${address.slice(-length)}`
}
