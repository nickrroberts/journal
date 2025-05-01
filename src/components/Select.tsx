export function Select({
    options,
    className = "",
    ...props
  }: {
    options: { label: string; value: string }[];
    className?: string;
  } & React.SelectHTMLAttributes<HTMLSelectElement>) {
    return (
      <div className={`relative inline-block w-full ${className}`}>
        <select
          className="
            appearance-none
            w-full
            bg-white
            border border-gray-300
            px-4 py-2 pr-8
            rounded-md
            text-gray-700
            focus:outline-none focus:ring-2 focus:ring-primary focus:border-primary
            transition
          "
          {...props}
        >
          {options.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        {/* custom arrow */}
        <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-3">
          <svg
            className="h-4 w-4 text-gray-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              d="M19 9l-7 7-7-7"
              strokeWidth={2}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>
      </div>
    );
  }