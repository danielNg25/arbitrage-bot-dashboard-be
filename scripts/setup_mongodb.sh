#!/bin/bash

# MongoDB Setup Script for Arbitrage Bot API
# This script helps you configure MongoDB connection settings

set -e

echo "🚀 MongoDB Configuration Setup for Arbitrage Bot API"
echo "=================================================="
echo ""

# Check if config.toml exists
if [ ! -f "config.toml" ]; then
    echo "❌ config.toml not found. Please run 'make config' first."
    exit 1
fi

echo "📋 Current configuration:"
echo "------------------------"
cat config.toml
echo ""

echo "🔧 MongoDB Configuration Options:"
echo "1. Local MongoDB (localhost:27017)"
echo "2. Remote MongoDB with authentication"
echo "3. MongoDB Atlas (cloud)"
echo "4. Custom connection string"
echo "5. Skip MongoDB setup (use dummy data)"
echo ""

read -p "Choose an option (1-5): " choice

case $choice in
    1)
        echo "📝 Setting up local MongoDB configuration..."
        # Update config.toml for local MongoDB
        sed -i.bak 's|uri = ".*"|uri = "mongodb://localhost:27017"|' config.toml
        sed -i.bak 's|database_name = ".*"|database_name = "arbitrage_bot"|' config.toml
        echo "✅ Local MongoDB configuration updated!"
        echo "   Make sure MongoDB is running on localhost:27017"
        ;;
    2)
        echo "📝 Setting up remote MongoDB configuration..."
        read -p "Enter MongoDB host: " host
        read -p "Enter MongoDB port (default: 27017): " port
        port=${port:-27017}
        read -p "Enter database name: " dbname
        read -p "Enter username: " username
        read -s -p "Enter password: " password
        echo ""
        read -p "Enter authentication database (default: admin): " authdb
        authdb=${authdb:-admin}
        
        # Update config.toml for remote MongoDB
        sed -i.bak "s|uri = \".*\"|uri = \"mongodb://${username}:${password}@${host}:${port}/${dbname}?authSource=${authdb}\"|" config.toml
        sed -i.bak "s|database_name = \".*\"|database_name = \"${dbname}\"|" config.toml
        echo "✅ Remote MongoDB configuration updated!"
        ;;
    3)
        echo "📝 Setting up MongoDB Atlas configuration..."
        read -p "Enter Atlas connection string: " atlas_uri
        read -p "Enter database name: " dbname
        
        # Update config.toml for MongoDB Atlas
        sed -i.bak "s|uri = \".*\"|uri = \"${atlas_uri}\"|" config.toml
        sed -i.bak "s|database_name = \".*\"|database_name = \"${dbname}\"|" config.toml
        echo "✅ MongoDB Atlas configuration updated!"
        ;;
    4)
        echo "📝 Setting up custom MongoDB configuration..."
        read -p "Enter custom connection string: " custom_uri
        read -p "Enter database name: " dbname
        
        # Update config.toml for custom MongoDB
        sed -i.bak "s|uri = \".*\"|uri = \"${custom_uri}\"|" config.toml
        sed -i.bak "s|database_name = \".*\"|database_name = \"${dbname}\"|" config.toml
        echo "✅ Custom MongoDB configuration updated!"
        ;;
    5)
        echo "⏭️  Skipping MongoDB setup. The API will use dummy data."
        echo "   You can configure MongoDB later by editing config.toml"
        ;;
    *)
        echo "❌ Invalid option. Please run the script again."
        exit 1
        ;;
esac

if [ $choice -ne 5 ]; then
    echo ""
    echo "🔍 Updated configuration:"
    echo "------------------------"
    cat config.toml
    echo ""
    
    echo "💡 Next steps:"
    echo "1. Ensure MongoDB is accessible with the configured connection string"
    echo "2. Test the connection by running: cargo run"
    echo "3. If you encounter connection issues, check:"
    echo "   - Network connectivity"
    echo "   - Authentication credentials"
    echo "   - Firewall settings"
    echo "   - MongoDB service status"
fi

echo ""
echo "🎉 Setup complete! You can now run the API with: cargo run"
