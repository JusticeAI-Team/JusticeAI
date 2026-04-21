import type { Directive } from 'vue'

const observer =
  typeof window !== 'undefined'
    ? new IntersectionObserver(
        (entries) => {
          entries.forEach((entry) => {
            if (entry.isIntersecting) {
              entry.target.classList.add('in')
              observer.unobserve(entry.target)
            }
          })
        },
        { threshold: 0.12 },
      )
    : null

export const revealDirective: Directive<HTMLElement, string | number | undefined> = {
  mounted(el, binding) {
    el.classList.add('reveal')

    if (binding.value != null) {
      el.style.transitionDelay = `${binding.value}ms`
    }

    observer?.observe(el)
  },
  unmounted(el) {
    observer?.unobserve(el)
  },
}
