<script lang="ts">
  let { checked, label, disabled, onchange }: {
    checked: boolean
    label: string
    disabled: boolean
    onchange: () => void
  } = $props()
</script>

<button
  type="button"
  role="switch"
  aria-label={label}
  aria-checked={checked}
  {disabled}
  onclick={onchange}
  class="relative h-6 w-11 shrink-0 rounded-full transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 disabled:opacity-50 {checked ? 'bg-green-500' : 'bg-gray-500'}"
>
  <span class="absolute top-0.5 left-0.5 size-5 rounded-full bg-white shadow transition-transform {checked ? 'translate-x-5' : ''}"></span>
</button>
