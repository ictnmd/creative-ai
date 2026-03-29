import { clsx, type ClassValue } from 'clsx';

/**
 * Utility for merging Tailwind CSS class names.
 * Wraps clsx to provide a consistent API.
 */
export function cn(...inputs: ClassValue[]): string {
	return clsx(inputs);
}
