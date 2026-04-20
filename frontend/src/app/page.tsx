import Link from 'next/link';

export default function Home() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[70vh] text-center">
      <h1 className="text-5xl font-bold text-gray-900 mb-6">Global Energy Metrics Platform</h1>
      <p className="text-xl text-gray-600 mb-8 max-w-2xl">
        Access 170,000+ records of clean, validated energy consumption, renewable trends, and climate metrics across 56 countries.
      </p>
      <Link
        href="/data"
        className="px-6 py-3 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition"
      >
        Explore the Data
      </Link>
    </div>
  );
}
