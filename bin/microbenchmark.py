#!/usr/bin/env python3
"""
Parsm Microbenchmark Suite

Tests the performance of parsm release binary across various data formats,
document sizes, and operations including field selection, filtering, and templating.

Prerequisites:
- Python 3.7+
- Virtual environment with dependencies installed
- parsm built in release mode

Setup:
    ./setup.sh
    source venv/bin/activate
    cargo build --release

Usage:
    python microbenchmark.py [options]
"""

import json
import csv
import time
import subprocess
import tempfile
import os
import sys
import statistics
from io import StringIO
from typing import Dict, List, Tuple, Any
from dataclasses import dataclass

# Try to import yaml, provide helpful error if missing
try:
    import yaml
except ImportError:
    print("Error: PyYAML is not installed.")
    print("Please run the setup script or install dependencies:")
    print("  ./setup.sh")
    print("  source venv/bin/activate")
    print("Or manually install: pip install PyYAML")
    sys.exit(1)


@dataclass
class BenchmarkResult:
    """Results from a single benchmark run"""
    operation: str
    format: str
    size: str
    avg_time: float
    min_time: float
    max_time: float
    std_dev: float
    iterations: int


class DataGenerator:
    """Generates test data in various formats and sizes"""
    
    @staticmethod
    def generate_json_objects(count: int) -> str:
        """Generate JSON array with specified number of simple objects"""
        objects = []
        for i in range(count):
            obj = {
                "id": f"obj_{i:06d}",
                "name": f"User {i}",
                "age": 20 + (i % 60),
                "active": i % 3 == 0,
                "score": round(50 + (i % 50) + (i * 0.1) % 10, 2),
                "email": f"user{i}@example.com",
                "department": ["Engineering", "Sales", "Marketing", "Support"][i % 4]
            }
            objects.append(obj)
        return json.dumps(objects, indent=2)
    
    @staticmethod
    def generate_csv_data(count: int) -> str:
        """Generate CSV data with specified number of rows"""
        output = StringIO()
        writer = csv.writer(output)
        
        # Header
        writer.writerow(['id', 'name', 'age', 'active', 'score', 'email', 'department', 'level'])
        
        # Data rows
        for i in range(count):
            writer.writerow([
                f"obj_{i:06d}",
                f"User {i}",
                20 + (i % 60),
                "true" if i % 3 == 0 else "false",
                round(50 + (i % 50) + (i * 0.1) % 10, 2),
                f"user{i}@example.com",
                ["Engineering", "Sales", "Marketing", "Support"][i % 4],
                ["junior", "mid", "senior"][i % 3]
            ])
        
        return output.getvalue()
    
    @staticmethod
    def generate_yaml_data(count: int) -> str:
        """Generate YAML data with specified number of simple objects"""
        objects = []
        for i in range(count):
            obj = {
                'id': f"obj_{i:06d}",
                'name': f"User {i}",
                'age': 20 + (i % 60),
                'active': i % 3 == 0,
                'score': round(50 + (i % 50) + (i * 0.1) % 10, 2),
                'email': f"user{i}@example.com",
                'department': ["Engineering", "Sales", "Marketing", "Support"][i % 4]
            }
            objects.append(obj)
        return yaml.dump(objects, default_flow_style=False)
    
    @staticmethod
    def generate_logfmt_data(count: int) -> str:
        """Generate logfmt data with specified number of lines"""
        lines = []
        for i in range(count):
            level = ["info", "warn", "error"][i % 3]
            service = ["api", "web", "worker", "db"][i % 4]
            duration = round(0.1 + (i % 100) * 0.01, 3)
            user_id = f"user_{i % 1000}"
            
            line = f'level={level} service={service} user_id={user_id} duration={duration}s msg="Request processed" status={200 + (i % 3)} request_id=req_{i:06d}'
            lines.append(line)
        
        return '\n'.join(lines)
    
    @staticmethod
    def generate_text_data(count: int) -> str:
        """Generate plain text data with specified number of lines"""
        lines = []
        for i in range(count):
            name = f"User{i}"
            age = 20 + (i % 60)
            dept = ["Eng", "Sales", "Mkt", "Support"][i % 4]
            score = round(50 + (i % 50) + (i * 0.1) % 10, 1)
            
            line = f"{name} {age} {dept} {score}"
            lines.append(line)
        
        return '\n'.join(lines)


class ParmsBenchmark:
    """Main benchmark runner for parsm"""
    
    def __init__(self, parsm_binary: str = "./target/release/parsm"):
        self.parsm_binary = parsm_binary
        self.results: List[BenchmarkResult] = []
        
        # Verify parsm binary exists
        if not os.path.exists(parsm_binary):
            raise FileNotFoundError(f"Parsm binary not found at {parsm_binary}")
    
    def run_parsm(self, input_data: str, args: List[str]) -> Tuple[float, bool]:
        """Run parsm with given input and arguments, return (execution_time, success)"""
        with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
            f.write(input_data)
            temp_file = f.name
        
        try:
            start_time = time.perf_counter()
            
            with open(temp_file, 'r') as f:
                result = subprocess.run(
                    [self.parsm_binary] + args,
                    stdin=f,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=30  # 30 second timeout
                )
            
            end_time = time.perf_counter()
            execution_time = end_time - start_time
            
            return execution_time, result.returncode == 0
            
        except subprocess.TimeoutExpired:
            return float('inf'), False
        finally:
            os.unlink(temp_file)
    
    def benchmark_operation(self, operation_name: str, format_name: str, size_name: str, 
                          input_data: str, args: List[str], iterations: int = 5) -> BenchmarkResult:
        """Benchmark a specific operation multiple times and return statistics"""
        times = []
        successful_runs = 0
        
        print(f"  Running {operation_name} on {format_name} ({size_name})...")
        
        for i in range(iterations):
            exec_time, success = self.run_parsm(input_data, args)
            if success and exec_time != float('inf'):
                times.append(exec_time)
                successful_runs += 1
            else:
                print(f"    Run {i+1} failed or timed out")
        
        if not times:
            print(f"    All runs failed for {operation_name}")
            return BenchmarkResult(
                operation=operation_name,
                format=format_name,
                size=size_name,
                avg_time=float('inf'),
                min_time=float('inf'),
                max_time=float('inf'),
                std_dev=float('inf'),
                iterations=0
            )
        
        result = BenchmarkResult(
            operation=operation_name,
            format=format_name,
            size=size_name,
            avg_time=statistics.mean(times),
            min_time=min(times),
            max_time=max(times),
            std_dev=statistics.stdev(times) if len(times) > 1 else 0.0,
            iterations=successful_runs
        )
        
        self.results.append(result)
        return result
    
    def run_format_benchmarks(self, format_name: str, data_small: str, data_medium: str):
        """Run all benchmark operations for a specific data format"""
        print(f"\nBenchmarking {format_name.upper()} format:")
        
        # Define test operations based on format
        if format_name == "json":
            operations = [
                ("field_selection", ["name"]),
                ("nested_field", ["profile.level"]),
                ("deep_nested", ["profile.location.city"]),
                ("filter_simple", ["age > 30"]),
                ("filter_complex", ["age > 30 && active == true"]),
                ("template_simple", ["${name}"]),
                ("template_complex", ["${name} (${age}) - ${department}"]),
                ("template_very_complex", ["ID: ${id}, User: ${name} (${age}), Dept: ${department}, Level: ${profile.level}, City: ${profile.location.city}"]),
                ("filter_and_template", ["age > 40", "${name}: ${profile.level}"]),
                ("cache_test_repeat", ["name"]),  # Test caching by repeating same operation
            ]
        elif format_name == "csv":
            operations = [
                ("field_selection", ["field_1"]),  # name column
                ("field_numeric", ["field_2"]),    # age column
                ("filter_simple", ["field_2 > \"30\""]),
                ("filter_complex", ["field_2 > \"30\" && field_3 == \"true\""]),
                ("template_simple", ["${field_1}"]),
                ("template_complex", ["${field_1} (${field_2}) - ${field_6}"]),
            ]
        elif format_name == "yaml":
            operations = [
                ("field_selection", ["name"]),
                ("nested_field", ["profile.level"]),
                ("filter_simple", ["age > 30"]),
                ("filter_complex", ["age > 30 && active == true"]),
                ("template_simple", ["${name}"]),
                ("template_complex", ["${name} (${age}) - ${department}"]),
            ]
        elif format_name == "logfmt":
            operations = [
                ("field_selection", ["level"]),
                ("field_service", ["service"]),
                ("filter_level", ["level == \"error\""]),
                ("filter_duration", ["duration > \"0.5s\""]),
                ("template_simple", ["${service}"]),
                ("template_complex", ["[${level}] ${service}: ${msg}"]),
            ]
        elif format_name == "text":
            operations = [
                ("field_selection", ["word_0"]),  # name
                ("field_numeric", ["word_1"]),    # age
                ("filter_simple", ["word_1 > \"30\""]),
                ("template_simple", ["${word_0}"]),
                ("template_complex", ["${word_0} (${word_1}) - ${word_2}"]),
            ]
        else:
            operations = [("basic_parse", [])]
        
        # Run benchmarks on small and medium datasets
        for op_name, args in operations:
            self.benchmark_operation(f"{op_name}_small", format_name, "small", data_small, args)
            self.benchmark_operation(f"{op_name}_medium", format_name, "medium", data_medium, args)
    
    def run_all_benchmarks(self):
        """Run comprehensive benchmark suite"""
        print("=== Parsm Microbenchmark Suite ===")
        print(f"Using parsm binary: {self.parsm_binary}")
        
        # Generate test data
        generator = DataGenerator()
        
        print("\nGenerating test data...")
        
        # Small datasets (100 records)
        json_small = generator.generate_json_objects(100)
        csv_small = generator.generate_csv_data(100)
        yaml_small = generator.generate_yaml_data(100)
        logfmt_small = generator.generate_logfmt_data(100)
        text_small = generator.generate_text_data(100)
        
        # Medium datasets (1,000 records)
        json_medium = generator.generate_json_objects(1000)
        csv_medium = generator.generate_csv_data(1000)
        yaml_medium = generator.generate_yaml_data(1000)
        logfmt_medium = generator.generate_logfmt_data(1000)
        text_medium = generator.generate_text_data(1000)
        
        # Large datasets (5,000 records) for performance testing
        print("Generating large datasets for performance testing...")
        json_large = generator.generate_json_objects(5000)
        csv_large = generator.generate_csv_data(5000)
        
        print(f"Generated datasets - Small: ~100 records, Medium: ~1,000 records, Large: ~5,000 records")
        
        # Run format-specific benchmarks
        formats_data = [
            ("json", json_small, json_medium),
            ("csv", csv_small, csv_medium),
            ("yaml", yaml_small, yaml_medium),
            ("logfmt", logfmt_small, logfmt_medium),
            ("text", text_small, text_medium),
        ]
        
        for format_name, small_data, medium_data in formats_data:
            self.run_format_benchmarks(format_name, small_data, medium_data)
        
        # Performance tests on large datasets
        print(f"\nRunning performance tests on large datasets:")
        performance_operations = [
            ("field_selection_large", "json", "large", json_large, ["name"]),
            ("template_simple_large", "json", "large", json_large, ["${name} (${age})"]),
            ("filter_simple_large", "json", "large", json_large, ["age > 30"]),
            ("csv_field_large", "csv", "large", csv_large, ["field_1"]),
            ("csv_filter_large", "csv", "large", csv_large, ["field_2 > \"30\""]),
        ]
        
        for op_name, format_name, size_name, data, args in performance_operations:
            self.benchmark_operation(op_name, format_name, size_name, data, args, iterations=3)
        
        # Special test with real JSON file if available
        if os.path.exists("/workspaces/parsm/7780.json"):
            print(f"\nBenchmarking real JSON file (7780.json):")
            with open("/workspaces/parsm/7780.json", 'r') as f:
                real_json = f.read()
            
            operations = [
                ("real_field_selection", ["Id"]),
                ("real_nested_field", ["State.Status"]),
                ("real_filter", ["State.Running == true"]),
                ("real_template", ["${Name}: ${State.Status}"]),
            ]
            
            for op_name, args in operations:
                self.benchmark_operation(op_name, "json_real", "real", real_json, args)
    
    def print_results(self):
        """Print benchmark results in a formatted table"""
        print("\n" + "="*100)
        print("BENCHMARK RESULTS")
        print("="*100)
        
        # Group results by format
        by_format = {}
        for result in self.results:
            if result.format not in by_format:
                by_format[result.format] = []
            by_format[result.format].append(result)
        
        for format_name, format_results in by_format.items():
            print(f"\n{format_name.upper()} Format Results:")
            print("-" * 80)
            print(f"{'Operation':<25} {'Size':<8} {'Avg Time (ms)':<15} {'Min (ms)':<10} {'Max (ms)':<10} {'StdDev':<8} {'Runs':<5}")
            print("-" * 80)
            
            for result in sorted(format_results, key=lambda x: (x.size, x.operation)):
                if result.avg_time == float('inf'):
                    avg_str = "FAILED"
                    min_str = "N/A"
                    max_str = "N/A"
                    std_str = "N/A"
                else:
                    avg_str = f"{result.avg_time * 1000:.2f}"
                    min_str = f"{result.min_time * 1000:.2f}"
                    max_str = f"{result.max_time * 1000:.2f}"
                    std_str = f"{result.std_dev * 1000:.2f}"
                
                print(f"{result.operation:<25} {result.size:<8} {avg_str:<15} {min_str:<10} {max_str:<10} {std_str:<8} {result.iterations:<5}")
    
    def export_csv_results(self, filename: str = "parsm_benchmark_results.csv"):
        """Export results to CSV file"""
        with open(filename, 'w', newline='') as csvfile:
            writer = csv.writer(csvfile)
            writer.writerow(['Operation', 'Format', 'Size', 'Avg_Time_ms', 'Min_Time_ms', 
                           'Max_Time_ms', 'StdDev_ms', 'Iterations'])
            
            for result in self.results:
                if result.avg_time == float('inf'):
                    writer.writerow([result.operation, result.format, result.size, 
                                   'FAILED', 'FAILED', 'FAILED', 'FAILED', result.iterations])
                else:
                    writer.writerow([result.operation, result.format, result.size, 
                                   f"{result.avg_time * 1000:.2f}", f"{result.min_time * 1000:.2f}",
                                   f"{result.max_time * 1000:.2f}", f"{result.std_dev * 1000:.2f}",
                                   result.iterations])
        
        print(f"\nResults exported to {filename}")


def main():
    """Main benchmark runner"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Parsm Microbenchmark Suite")
    parser.add_argument("--parsm-binary", default="./target/release/parsm",
                       help="Path to parsm binary (default: ./target/release/parsm)")
    parser.add_argument("--export-csv", action="store_true",
                       help="Export results to CSV file")
    parser.add_argument("--iterations", type=int, default=5,
                       help="Number of iterations per benchmark (default: 5)")
    
    args = parser.parse_args()
    
    try:
        benchmark = ParmsBenchmark(args.parsm_binary)
        benchmark.run_all_benchmarks()
        benchmark.print_results()
        
        if args.export_csv:
            benchmark.export_csv_results()
            
    except FileNotFoundError as e:
        print(f"Error: {e}")
        print("Make sure to build parsm in release mode first:")
        print("  cargo build --release")
        return 1
    except KeyboardInterrupt:
        print("\nBenchmark interrupted by user")
        return 1
    except Exception as e:
        print(f"Unexpected error: {e}")
        return 1
    
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())